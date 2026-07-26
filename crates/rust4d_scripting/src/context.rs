//! Per-call context threaded through Lua `app_data`
//!
//! The `ScriptEngine` does not own or borrow the engine's `hecs::World`,
//! input state, or audio engine. Instead, `ScriptSystem` registers these into
//! the Lua VM's `app_data` for the lifetime of each lifecycle callback and
//! clears them afterwards. This keeps the `&mut World` borrow inside the call
//! boundary and avoids storing `Send`-requiring free pointers across calls.
//!
//! See the Wave 5 plan ("pass `&mut hecs::World` into the call, do not store
//! it") and `src/systems/script.rs`.
//!
//! # Safety
//!
//! [`WorldRef`] and [`AudioRef`] hold raw pointers that are only valid for the
//! duration of a single lifecycle call. `ScriptSystem` guarantees the referent
//! outlives the call and that no other Rust code touches it while the Lua
//! callback is running. Bindings dereference the pointers under that invariant.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use rust4d_audio::AudioEngine4D;
use rust4d_core::World;

/// Per-call handle to the live `rust4d_core::World`.
///
/// `!Send + !Sync`: only valid on the main thread for the call's lifetime.
pub struct WorldRef(pub(crate) *mut World);

impl WorldRef {
    /// Construct from a mutable borrow. The caller (ScriptSystem) must keep
    /// the world alive and unaliased for the duration of the Lua call.
    pub fn new(world: &mut World) -> Self {
        Self(world as *mut World)
    }

    /// # Safety
    ///
    /// Caller must ensure the pointer is valid and not mutably borrowed
    /// elsewhere for the duration of the returned reference.
    #[allow(clippy::mut_from_ref)]
    #[inline]
    pub unsafe fn as_mut(&self) -> &mut World {
        unsafe { &mut *self.0 }
    }
}

/// Per-call handle to an optional `AudioEngine4D`.
///
/// `None` when audio is disabled or unavailable; audio bindings no-op in that
/// case. `!Send + !Sync`.
pub struct AudioRef(pub(crate) Option<*const AudioEngine4D>);

impl AudioRef {
    pub fn new(audio: Option<&AudioEngine4D>) -> Self {
        Self(audio.map(|a| a as *const AudioEngine4D))
    }

    /// # Safety
    ///
    /// Caller must ensure the engine, when present, outlives the use of the
    /// returned reference.
    #[inline]
    pub unsafe fn as_ref(&self) -> Option<&AudioEngine4D> {
        self.0.map(|p| unsafe { &*p })
    }
}

/// Mutation flag set by ECS bindings when scripts modify the world.
///
/// `ScriptSystem` reads (and resets) this after each update to decide whether
/// the geometry cache needs rebuilding.
#[derive(Debug, Default)]
pub struct ScriptMutations {
    pub geometry_dirty: Cell<bool>,
}

impl ScriptMutations {
    pub fn mark_dirty(&self) {
        self.geometry_dirty.set(true);
    }

    pub fn is_dirty(&self) -> bool {
        self.geometry_dirty.get()
    }
}

/// A snapshot of input state for one frame.
///
/// Populated by the app from raw winit events and the action map, then
/// registered as `app_data` for the duration of `on_update` so the `input`
/// bindings can read it. "Just pressed" sets are cleared at the end of each
/// frame by the app.
#[derive(Debug, Clone, Default)]
pub struct InputSnapshot {
    /// Canonical key names currently held down (e.g. `"W"`, `"Space"`).
    pub pressed_keys: HashSet<String>,
    /// Keys pressed this frame (rising edge), cleared after the frame.
    pub just_pressed_keys: HashSet<String>,
    /// Mouse movement accumulated since the last frame (dx, dy).
    pub mouse_delta: (f32, f32),
    /// Mouse position in window pixels (x, y).
    pub mouse_position: (f32, f32),
    /// Canonical mouse button names currently held (`"left"`, `"right"`, `"middle"`).
    pub pressed_mouse: HashSet<String>,
    /// Mouse buttons pressed this frame (rising edge).
    pub just_pressed_mouse: HashSet<String>,
    /// Named action axes in `[-1, 1]` (e.g. `"move_forward"`, `"strafe"`).
    pub actions: HashMap<String, f32>,
}

impl InputSnapshot {
    /// Create an empty snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a key press (held + rising edge).
    pub fn press_key(&mut self, name: impl Into<String>) {
        let name = normalize_key(&name.into());
        if self.pressed_keys.insert(name.clone()) {
            self.just_pressed_keys.insert(name);
        }
    }

    /// Record a key release.
    pub fn release_key(&mut self, name: impl Into<String>) {
        self.pressed_keys.remove(&normalize_key(&name.into()));
    }

    /// Record a mouse button press (held + rising edge).
    pub fn press_mouse(&mut self, name: impl Into<String>) {
        let name = name.into().to_lowercase();
        if self.pressed_mouse.insert(name.clone()) {
            self.just_pressed_mouse.insert(name);
        }
    }

    /// Record a mouse button release.
    pub fn release_mouse(&mut self, name: impl Into<String>) {
        self.pressed_mouse.remove(&name.into().to_lowercase());
    }

    /// Accumulate mouse motion delta.
    pub fn add_mouse_motion(&mut self, dx: f32, dy: f32) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    /// Set the mouse position.
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    /// Set a named action axis value.
    pub fn set_action(&mut self, name: impl Into<String>, value: f32) {
        self.actions.insert(name.into(), value);
    }

    /// Clear per-frame (rising-edge and delta) state. Call after the frame.
    pub fn end_frame(&mut self) {
        self.just_pressed_keys.clear();
        self.just_pressed_mouse.clear();
        self.mouse_delta = (0.0, 0.0);
    }

    /// Whether a key is currently held (case-insensitive name match).
    pub fn is_key_pressed(&self, name: &str) -> bool {
        self.pressed_keys.contains(&normalize_key(name))
    }

    /// Whether a key was pressed this frame (case-insensitive name match).
    pub fn is_key_just_pressed(&self, name: &str) -> bool {
        self.just_pressed_keys.contains(&normalize_key(name))
    }

    /// Whether a mouse button is held.
    pub fn is_mouse_pressed(&self, name: &str) -> bool {
        self.pressed_mouse.contains(&name.to_lowercase())
    }

    /// Whether a mouse button was pressed this frame.
    pub fn is_mouse_just_pressed(&self, name: &str) -> bool {
        self.just_pressed_mouse.contains(&name.to_lowercase())
    }

    /// Axis value for a key pair: +1 if positive held, -1 if negative held,
    /// 0 otherwise (0 if both held).
    pub fn key_axis(&self, positive: &str, negative: &str) -> f32 {
        let pos = self.is_key_pressed(positive) as i32 as f32;
        let neg = self.is_key_pressed(negative) as i32 as f32;
        pos - neg
    }
}

fn normalize_key(name: &str) -> String {
    // Letters single-uppercase; everything else left as-is for stable HashSet
    // lookups regardless of how the script spelled the key.
    if name.len() == 1 {
        name.to_uppercase()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_snapshot_press_release() {
        let mut snap = InputSnapshot::new();
        snap.press_key("w");
        assert!(snap.is_key_pressed("W"));
        assert!(snap.is_key_just_pressed("W"));
        snap.end_frame();
        assert!(snap.is_key_pressed("W"));
        assert!(!snap.is_key_just_pressed("W"));
        snap.release_key("w");
        assert!(!snap.is_key_pressed("W"));
    }

    #[test]
    fn test_input_snapshot_axis() {
        let mut snap = InputSnapshot::new();
        snap.press_key("d");
        assert_eq!(snap.key_axis("D", "A"), 1.0);
        snap.press_key("a");
        assert_eq!(snap.key_axis("D", "A"), 0.0);
        snap.release_key("d");
        assert_eq!(snap.key_axis("D", "A"), -1.0);
    }

    #[test]
    fn test_input_snapshot_mouse() {
        let mut snap = InputSnapshot::new();
        snap.press_mouse("Left");
        assert!(snap.is_mouse_pressed("left"));
        assert!(snap.is_mouse_just_pressed("left"));
        snap.end_frame();
        assert!(!snap.is_mouse_just_pressed("left"));
    }

    #[test]
    fn test_input_snapshot_mouse_delta_accumulates() {
        let mut snap = InputSnapshot::new();
        snap.add_mouse_motion(1.0, 2.0);
        snap.add_mouse_motion(3.0, 4.0);
        assert_eq!(snap.mouse_delta, (4.0, 6.0));
        snap.end_frame();
        assert_eq!(snap.mouse_delta, (0.0, 0.0));
    }

    #[test]
    fn test_script_mutations_flag() {
        let m = ScriptMutations::default();
        assert!(!m.is_dirty());
        m.mark_dirty();
        assert!(m.is_dirty());
    }
}
