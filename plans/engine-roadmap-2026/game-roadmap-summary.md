# Rust4D-Shooter: Game Repo Roadmap

**Date**: 2026-01-31
**Type**: Consolidated game-side implementation plan
**Companion to**: Engine roadmap (agent reports F, P1-P5)

---

## Overview

Rust4D-Shooter is a 4D boomer shooter distributed via Steam. It lives in a separate repository from the Rust4D engine and depends on the engine crates via git URL. The game implements all gameplay-specific systems -- weapons, enemies, AI, levels, HUD, menus -- on top of the generic engine APIs.

This document consolidates every game-side work item identified across all engine roadmap agents (F, P1, P2, P3, P4, P5) and the engine/game split plan, organized into phases that align with engine delivery milestones.

**Key principle**: The engine is a generic 4D game engine library. The game repo owns all boomer-shooter-specific logic. Health, weapons, enemy types, level scripts, HUD widgets, and menus are all game code.

---

## Phase 0: Game Repo Setup

**Depends on**: Engine split plan Phases 1-4 complete (ECS migration, rust4d_game crate, pluggable scene instantiation)
**Estimated effort**: 1-2 sessions

### What Moves from Engine to Game

The following files move from the engine repo (`Rust4D/`) to the game repo (`Rust4D-Shooter/`):

| Source (Engine) | Destination (Game) |
|---|---|
| `src/main.rs` | `src/main.rs` (winit app, event loop) |
| `src/config.rs` | `src/config.rs` (game configuration) |
| `src/systems/simulation.rs` | `src/systems/simulation.rs` |
| `src/systems/render.rs` | `src/systems/render.rs` |
| `src/systems/window.rs` | `src/systems/window.rs` |
| `src/input/input_mapper.rs` | `src/input/input_mapper.rs` |
| `scenes/` | `scenes/` (RON scene files) |
| `config/` | `config/` (TOML config files) |

### Repository Structure

```
Rust4D-Shooter/
  Cargo.toml                  # Git URL deps on rust4d_* crates
  .cargo/config.toml          # Local path overrides for dev iteration
  src/
    main.rs                   # Game entry point (winit app)
    config.rs                 # Game configuration structs
    systems/
      mod.rs
      simulation.rs           # Game simulation loop
      render.rs               # Game render orchestration
      window.rs               # Window management
    input/
      mod.rs
      input_mapper.rs         # Key-to-action mapping
    player/
      mod.rs                  # CharacterController4D usage
  scenes/                     # Game scenes (RON)
  config/                     # Game config (TOML)
  assets/
    sounds/                   # WAV/OGG sound effects
    textures/                 # PNG/JPG textures
    sprites/                  # Enemy sprite sheets
```

### Cargo.toml Setup

```toml
# Rust4D-Shooter/Cargo.toml (committed, works on any machine)
[dependencies]
rust4d_game = { git = "https://github.com/Lemon9247/Rust4D.git" }
rust4d_render = { git = "https://github.com/Lemon9247/Rust4D.git" }
rust4d_input = { git = "https://github.com/Lemon9247/Rust4D.git" }
rust4d_audio = { git = "https://github.com/Lemon9247/Rust4D.git" }
```

```toml
# Rust4D-Shooter/.cargo/config.toml (for local iteration)
[patch.'https://github.com/Lemon9247/Rust4D.git']
rust4d_game = { path = "../Rust4D/crates/rust4d_game" }
rust4d_render = { path = "../Rust4D/crates/rust4d_render" }
rust4d_input = { path = "../Rust4D/crates/rust4d_input" }
rust4d_audio = { path = "../Rust4D/crates/rust4d_audio" }
```

### Adaptation Work

All moved code must be adapted to the new engine APIs:

1. **ECS queries** instead of monolithic Entity iteration
2. **CharacterController4D** instead of direct `PhysicsWorld.player_*()` methods
3. **rust4d_game scene helpers** instead of hardcoded `from_template()` tag-based physics setup
4. **InputMap with abstract actions** instead of raw KeyCode handling
5. **PhysicsWorld::update(dt)** (fixed timestep accumulator) instead of raw `step(dt)`
6. **Movement normalization** -- CharacterController4D handles diagonal normalization internally (fixes the sqrt(3) speed bug in 4D)

### Verification

- `cargo run` in game repo starts the tech demo with identical behavior to pre-split
- `cargo test` in game repo passes all game-specific tests
- Game builds against engine crates via git URL (CI)

---

## Phase 1: Combat Core (Game-Side)

**Depends on**: Engine Phase 1 complete (raycasting, collision events, trigger detection)
**Estimated effort**: 2-3 sessions

This is the core gameplay loop: the player can shoot things and things can take damage.

### 1.1 Health/Damage System

**Source**: Agent P1 -- "Health/Damage is 100% game-side. The engine needs nothing."

The engine provides ECS, collision events, and raycasting. The game defines all combat components.

```rust
// Game-defined components
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self { Self { current: max, max } }
    pub fn apply_damage(&mut self, amount: f32) { self.current = (self.current - amount).max(0.0); }
    pub fn is_dead(&self) -> bool { self.current <= 0.0 }
    pub fn heal(&mut self, amount: f32) { self.current = (self.current + amount).min(self.max); }
}

pub struct DealsDamage {
    pub amount: f32,
    pub damage_type: DamageType,
}

pub enum DamageType {
    Hitscan,
    Projectile,
    Explosion,
    Melee,
}
```

**Tasks**:
- [ ] Define `Health` component (current HP, max HP, heal, damage, death check)
- [ ] Define `DealsDamage` component for projectiles and damage sources
- [ ] Implement damage system: process collision events from `physics_world.drain_collision_events()`, check if either entity has `DealsDamage` + other has `Health`, apply damage
- [ ] Implement hitscan damage: use engine's `PhysicsWorld::raycast()` to detect hits, apply damage to entities with `Health`
- [ ] Death handling: when `Health.is_dead()`, fire death event, begin death animation, schedule despawn
- [ ] Player death: game state transition to "dead" screen

### 1.2 Weapon System (Basic)

**Source**: Agent P2 -- "Weapon system is 100% game-side."

```rust
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub ammo_current: u32,
    pub ammo_max: u32,
    pub fire_rate: f32,       // Shots per second
    pub fire_cooldown: f32,   // Time until next shot
    pub damage: f32,
}

pub enum WeaponType {
    Shotgun,        // Hitscan, spread, short range
    RocketLauncher, // Projectile, explosive, splash damage
}

pub struct WeaponInventory {
    pub weapons: Vec<Weapon>,
    pub current_index: usize,
}
```

**Tasks**:
- [ ] Define `Weapon` component (type, ammo, fire rate, damage)
- [ ] Define `WeaponInventory` for weapon switching
- [ ] Implement hitscan shotgun:
  - Cast multiple rays (spread pattern) using `PhysicsWorld::raycast()`
  - Use `CollisionLayer::ENEMY` mask to detect hits
  - Apply damage with distance falloff
- [ ] Implement projectile rocket:
  - Spawn projectile entity with velocity, `DealsDamage` component, `CollisionFilter::projectile()`
  - On collision event: use `PhysicsWorld::query_area_effect()` for hyperspherical explosion radius
  - Apply splash damage with linear falloff
- [ ] Weapon switching (number keys or scroll wheel)
- [ ] Ammo tracking and depletion
- [ ] Fire rate limiting (cooldown timer)

### 1.3 Game Event Bus

**Source**: Agent P1 -- "The engine produces collision event DATA. The event bus belongs in rust4d_game."

The simulation loop translates engine collision events into game events:

```rust
pub enum GameEvent {
    Damage { target: hecs::Entity, amount: f32, source: hecs::Entity },
    Death { entity: hecs::Entity },
    Pickup { entity: hecs::Entity, pickup_type: PickupType },
    TriggerEnter { trigger: hecs::Entity, entity: hecs::Entity },
    TriggerExit { trigger: hecs::Entity, entity: hecs::Entity },
    WeaponFired { weapon_type: WeaponType, position: Vec4 },
    EnemyAlert { enemy: hecs::Entity },
}
```

**Tasks**:
- [ ] Set up `EventBus` from `rust4d_game` (or implement a simple typed channel)
- [ ] Each frame: drain `PhysicsWorld::drain_collision_events()`, translate to game events
- [ ] Dispatch game events to relevant systems (damage, audio, particles, AI)

### Engine APIs Used

| Engine API | Game Usage |
|---|---|
| `PhysicsWorld::raycast()` | Hitscan weapon hit detection |
| `PhysicsWorld::raycast_nearest()` | Single-target hitscan |
| `PhysicsWorld::drain_collision_events()` | Collision/trigger event polling |
| `PhysicsWorld::query_area_effect()` | Explosion splash damage |
| `CollisionLayer::ENEMY`, `PLAYER`, `PROJECTILE` | Layer filtering for weapons |
| `rust4d_game::EventBus` | Game event dispatch |
| `CharacterController4D` | Player movement, jump |

---

## Phase 2: Weapons and Feedback (Game-Side)

**Depends on**: Engine Phase 2 complete (audio, egui overlay, particles, screen effects)
**Estimated effort**: 3-4 sessions

This phase makes combat feel good. All specific widget implementations, sound triggers, and effect presets are game code.

### 2.1 Specific Weapon Implementations

**Source**: Agent P2 -- weapon definitions are game-side.

- [ ] Shotgun: 8-pellet spread, hitscan, effective range ~15 units, damage falloff with distance
- [ ] Rocket launcher: projectile speed 30 units/s, explosion radius 5 units (4D hypersphere), 100 base damage with linear falloff
- [ ] Weapon pickup entities in levels (see Phase 4 pickups)
- [ ] Weapon switching animation/delay

### 2.2 HUD Widgets

**Source**: Agent P2 -- "Specific HUD widgets (health bar, ammo counter, crosshair) are game-side."

All HUD is built using the engine's `OverlayRenderer` providing an `egui::Context`.

```rust
struct GameHud;

impl GameHud {
    fn draw(&self, ctx: &egui::Context, state: &GameState) {
        // Bottom-left: Health bar
        // Bottom-right: Ammo counter
        // Center: Crosshair
        // Top-right: W-position indicator
        // Top-left: Weapon name
    }
}
```

**Tasks**:
- [ ] Health bar (bottom-left, red, shows current/max HP)
- [ ] Ammo counter (bottom-right, shows current ammo / max ammo for active weapon)
- [ ] Crosshair (center screen, simple + or dot)
- [ ] W-position indicator (top-right, shows current W-slice position -- critical for 4D gameplay awareness)
- [ ] Active weapon name/icon display
- [ ] Damage direction indicator (flash on screen edge showing hit direction)

### 2.3 Screen Shake Integration

**Source**: Agent P2 -- "ScreenShake is game-side (camera offset, not post-processing). Lives in rust4d_game."

```rust
// Using rust4d_game::ScreenShake
fn on_weapon_fire(&mut self, weapon: WeaponType) {
    match weapon {
        WeaponType::Shotgun => self.screen_shake.add_trauma(0.3),
        WeaponType::RocketLauncher => self.screen_shake.add_trauma(0.5),
    }
}

fn on_damage_taken(&mut self, amount: f32) {
    self.screen_shake.add_trauma((amount / 50.0).min(0.8));
}
```

**Tasks**:
- [ ] Wire screen shake to weapon fire events (trauma amount per weapon type)
- [ ] Wire screen shake to damage taken events
- [ ] Apply `ScreenShake::get_offset()` to Camera4D before computing view matrices

### 2.4 Damage Flash via egui Overlay

**Source**: Agent P2 -- "Damage flash via egui overlay -- no post-processing pipeline needed."

```rust
fn draw_damage_flash(ctx: &egui::Context, intensity: f32) {
    if intensity > 0.0 {
        egui::Area::new("damage_flash")
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let alpha = (intensity * 128.0) as u8;
                let color = egui::Color32::from_rgba_unmultiplied(255, 0, 0, alpha);
                let size = ui.available_size();
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size),
                    0.0, color,
                );
            });
    }
}
```

**Tasks**:
- [ ] Implement `TimedEffect`-based damage flash (red overlay, fades over 0.3s)
- [ ] Implement pickup flash (brief green or blue flash on item pickup)
- [ ] Muzzle flash lighting boost (temporarily increase `RenderUniforms.ambient_strength` for 1-2 frames)

### 2.5 Sound Effect Triggering

**Source**: Agent P2 -- "Which sounds to play when (gunfire, pickup, enemy death) is game-side."

```rust
struct GameAudio {
    engine: AudioEngine4D,
    shotgun_fire: SoundHandle,
    rocket_fire: SoundHandle,
    rocket_explode: SoundHandle,
    enemy_pain: SoundHandle,
    enemy_death: SoundHandle,
    pickup_health: SoundHandle,
    pickup_ammo: SoundHandle,
    door_open: SoundHandle,
    music_level1: SoundHandle,
}
```

**Tasks**:
- [ ] Load all game sound assets at startup via `AudioEngine4D::load_sound()`
- [ ] Wire weapon fire events to spatial sound playback
- [ ] Wire damage/death events to enemy pain/death sounds
- [ ] Wire pickup events to pickup sounds
- [ ] Update listener position each frame via `AudioEngine4D::update_listener()`
- [ ] Background music management (per-level music tracks)

### 2.6 Particle Effect Presets

**Source**: Agent P2/P3 -- effect configurations are game-defined.

**Tasks**:
- [ ] Define muzzle flash config (`ParticleEmitterConfig`: 20 particles, 0.1s lifetime, bright yellow-white, additive blend)
- [ ] Define blood splatter config (30 particles, 0.5s, dark red, gravity 15.0, alpha blend)
- [ ] Define explosion config (100 particles, 1.0s, orange-to-gray, full sphere spread, drag 0.5)
- [ ] Define impact sparks config (15 particles, 0.2s, white-yellow, narrow spread)
- [ ] Wire particle spawning to weapon fire, hit, and explosion events

### Engine APIs Used

| Engine API | Game Usage |
|---|---|
| `OverlayRenderer` / `egui::Context` | HUD widgets, damage flash |
| `rust4d_game::ScreenShake` | Camera shake on fire/hit |
| `rust4d_game::TimedEffect` | Damage flash, muzzle flash timing |
| `AudioEngine4D` | Sound loading and playback |
| `AudioEngine4D::play_spatial()` | Positional SFX |
| `ParticleSystem::spawn_burst()` | Muzzle flash, blood, explosions |
| `ParticleEmitterConfig` | Effect preset definitions |

---

## Phase 3: Enemies (Game-Side)

**Depends on**: Engine Phase 3 complete (sprites/billboards, FSM, spatial queries, particles)
**Estimated effort**: 3-4 sessions

### 3.1 Enemy Types

**Source**: Agent P3 -- "3 enemy types are entirely game-level."

```rust
struct EnemyDef {
    name: &'static str,
    health: f32,
    move_speed: f32,
    attack_damage: f32,
    attack_range: f32,
    sight_range: f32,
    pain_chance: f32,
    w_behavior: WBehavior,
    // Animation frame ranges for sprite sheet
    anim_idle: (u32, u32),
    anim_walk: (u32, u32),
    anim_attack: (u32, u32),
    anim_pain: (u32, u32),
    anim_death: (u32, u32),
}

enum WBehavior {
    Standard,                          // Chases in all 4D
    WPhaser { phase_cooldown: f32, phase_duration: f32 },  // Phases between W-slices
    WFlanker { preferred_w_offset: f32 },                   // Approaches from adjacent W
}
```

**Enemy 1: Melee Rusher**
- Health: 50, Speed: 15 (fast), Damage: 20, Range: 1.5
- Behavior: Charges straight at player, simple but dangerous in groups
- W-behavior: Standard (chases in all 4 dimensions)

**Enemy 2: Projectile Gunner**
- Health: 80, Speed: 6 (slow), Damage: 15, Range: 25
- Behavior: Keeps distance, fires projectiles at player
- W-behavior: Standard

**Enemy 3: W-Phaser**
- Health: 60, Speed: 10, Damage: 25, Range: 2.0
- Behavior: Phases between W-slices to ambush, hard to stagger (pain_chance 0.2)
- W-behavior: WPhaser (shifts W-position during combat, fading in/out via sprite W-distance system)

**Tasks**:
- [ ] Define `EnemyDef` data struct for each enemy type with all stats
- [ ] Create sprite sheets for each enemy type (or placeholder colored billboards)
- [ ] Register enemy archetypes for spawning

### 3.2 AI State Machines

**Source**: Agent P3 -- "All AI logic is game-side. Engine FSM is ~30 lines of code."

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EnemyState { Idle, Chase, Attack, Pain, Dead }

struct EnemyAI {
    fsm: StateMachine<EnemyState>,  // From rust4d_game
    body_key: BodyKey,
    target: Option<hecs::Entity>,
    attack_range: f32,
    sight_range: f32,
    pain_chance: f32,
    attack_cooldown: f32,
    move_speed: f32,
    attack_damage: f32,
    w_behavior: WBehavior,
}
```

**State transitions**:
- **Idle**: Check for player within sight_range using `PhysicsWorld::query_sphere()`. If found and LOS check passes via `PhysicsWorld::line_of_sight()`, transition to Chase.
- **Chase**: Move toward player (set velocity = direction * move_speed). If within attack_range, transition to Attack. If LOS lost for >3s, return to Idle.
- **Attack**: Execute attack (melee hit or spawn projectile). After attack animation, cooldown, then return to Chase.
- **Pain**: On receiving damage (random check vs pain_chance), briefly stagger. Return to Chase after pain animation.
- **Dead**: Play death animation. Despawn entity after animation completes.

**Tasks**:
- [ ] Implement `EnemyAI` struct with `StateMachine<EnemyState>` from engine
- [ ] Implement Idle state: spatial query for player detection + LOS check
- [ ] Implement Chase state: steer toward player using `direction.normalized() * move_speed`
- [ ] Implement Attack state: melee hit (collision event) or projectile spawn
- [ ] Implement Pain state: stagger animation, brief pause
- [ ] Implement Dead state: death animation, despawn timer
- [ ] Wire sprite animation frames to AI states

### 3.3 Enemy Spawning and Wave Management

**Tasks**:
- [ ] Define enemy spawn points in RON scene files (tag: "enemy_spawn", with enemy type metadata)
- [ ] Implement spawn system: read spawn point entities, instantiate enemy entities with ECS components (Health, EnemyAI, SpriteAnimation, PhysicsBody)
- [ ] Wave management: trigger-based spawning (player enters area -> spawn wave of enemies)
- [ ] Enemy count tracking for level completion triggers

### 3.4 W-Phasing Enemy Behavior

**Source**: Agent P3 -- "W-phasing is a game-level behavior using engine sprite W-fade."

The engine's sprite W-distance fade handles the visual seamlessly. The game controls the W-coordinate:

```rust
fn update_w_phaser(&mut self, dt: f32, body: &mut RigidBody4D, player_w: f32) {
    self.phase_timer -= dt;
    if self.phase_timer <= 0.0 {
        // Phase to adjacent W-slice
        let target_w = player_w + self.preferred_w_offset;
        body.position.w = target_w;
        self.phase_timer = self.phase_cooldown;
    }
}
```

**Tasks**:
- [ ] Implement W-phaser state machine extension (phase-in, attack, phase-out cycle)
- [ ] W-flanker approach logic (approach from W offset, then snap to player's W-slice for attack)
- [ ] Visual/audio cues for W-phasing (particle trail at projected position, spatial audio cue)

### 3.5 Specific Particle Effects for Enemies

**Tasks**:
- [ ] Blood splatter on enemy hit (burst at hit point using blood_config)
- [ ] Enemy death explosion particles
- [ ] W-phaser "shimmer" particles (subtle glow at projected position when partially visible)
- [ ] Projectile trail particles for enemy projectiles

### Engine APIs Used

| Engine API | Game Usage |
|---|---|
| `StateMachine<S>` (rust4d_game) | Enemy AI state management |
| `PhysicsWorld::query_sphere()` | Enemy awareness (player detection) |
| `PhysicsWorld::line_of_sight()` | Can enemy see the player? |
| `PhysicsWorld::apply_impulse()` | Knockback from explosions |
| `SpriteBatch::add_sprite_4d()` | Enemy rendering with W-fade |
| `SpriteAnimation` | Frame-by-frame enemy animation |
| `ParticleSystem::spawn_burst()` | Blood, death effects |
| `CollisionFilter::enemy()` | Enemy collision layer setup |

---

## Phase 4: Level Design (Game-Side)

**Depends on**: Engine Phase 4 complete (shapes, triggers, tweens, RON preview)
**Estimated effort**: 2-4 sessions

### 4.1 Scene RON Files for Levels

**Source**: Agent P4 -- "RON scene files for levels are game-side."

The game creates RON scene files using the engine's shape types and trigger system.

**Tasks**:
- [ ] Design Level 1 layout (tutorial/intro) using engine shape types:
  - `Hyperprism4D` for walls (thin in one axis), floors, platforms
  - `Hypersphere4D` for decorative pillars/objects
  - W-layered rooms at different W-coordinates
- [ ] Define spawn points, enemy placements, pickup locations
- [ ] Create trigger zones for doors, pickups, enemy spawns
- [ ] Use engine's RON preview tool for iterative level design

### 4.2 Door/Elevator/Key Mechanics

**Source**: Agent P4 -- "Door logic, key/door pairing, and elevator behavior are game-side."

```rust
struct Door {
    closed_position: Vec4,
    open_position: Vec4,
    open_duration: f32,
    state: DoorState,
    required_key: Option<KeyColor>,
}

enum DoorState { Closed, Opening, Open(f32), Closing }

enum KeyColor { Red, Blue, Yellow }

struct Elevator {
    waypoints: Vec<Vec4>,
    current_waypoint: usize,
    speed: f32,
    pause_at_each: f32,
}
```

**Tasks**:
- [ ] Implement `Door` component and door system:
  - Listen for `TriggerEnter` events from `drain_collision_events()`
  - Check key requirement against player inventory
  - Use engine `TweenManager::tween_position()` for smooth open/close animation
- [ ] Implement `Elevator` component and elevator system:
  - Cycle through waypoints using tween system
  - Pause at each waypoint for configurable duration
- [ ] Implement key/door color system (Red/Blue/Yellow keys unlock matching doors)
- [ ] Wire doors/elevators to declarative trigger actions in RON:
  ```ron
  TriggerDef(
      name: "door_trigger_1",
      zone: AABB(center: (...), half_extents: (...)),
      detects: [Player],
      actions: [TweenPosition(target_entity: "door_1", to: (...), duration: 1.5, easing: EaseInOutQuad)],
      repeat: Once,
  )
  ```

### 4.3 Pickup System

**Source**: Agent P4 -- "Pickup system is primarily game-side."

```rust
struct Pickup {
    pickup_type: PickupType,
    amount: f32,
    respawn_time: Option<f32>,
}

enum PickupType {
    Health,
    Ammo(WeaponType),
    Weapon(WeaponType),
    Key(KeyColor),
}
```

**Tasks**:
- [ ] Implement `Pickup` component
- [ ] Create pickup entities with `CollisionFilter::trigger(CollisionLayer::PLAYER)` and visible sprite/shape
- [ ] On trigger enter: apply pickup effect (heal, add ammo, add weapon, add key)
- [ ] Despawn pickup on collection
- [ ] Optional: respawn timer for multiplayer/replayability
- [ ] Pickup bob/rotate animation (simple sine wave on Y position)
- [ ] Audio and particle feedback on pickup

### 4.4 W-Layered Room Design

**Source**: Agent P4 -- "W-portals are just trigger zones that tween the player's W-coordinate."

**Tasks**:
- [ ] Design rooms at different W-coordinates connected by W-portal triggers
- [ ] Implement W-portal triggers in RON:
  ```ron
  TriggerDef(
      name: "w_portal_0_to_5",
      zone: AABB(center: (10.0, 1.0, 0.0, 0.5), half_extents: (1.0, 2.0, 1.0, 0.5)),
      detects: [Player],
      actions: [GameEvent("shift_player_w_to_5")],
      repeat: Always,
  )
  ```
- [ ] Game-side handler for `GameEvent("shift_player_w_to_N")` that tweens the player's W-coordinate
- [ ] Visual cues at W-portal locations (particle effects, color shift)

### 4.5 Level Scripting

Using the engine's declarative trigger system for 80% of needs:
- [ ] Secret doors: interaction trigger reveals hidden passage
- [ ] Trap triggers: ceiling crush, floor drop
- [ ] Enemy spawn triggers: player enters area -> spawn wave
- [ ] Completion triggers: all enemies dead -> open exit door
- [ ] W-portal triggers: shift player to different W-layer

For the remaining 20%: custom Rust game systems.

### Engine APIs Used

| Engine API | Game Usage |
|---|---|
| `TriggerDef` / declarative triggers | RON-defined level scripting |
| `TweenManager` / `Tween<T>` | Door/elevator smooth movement |
| `EasingFunction` | Smooth door/elevator easing |
| `TriggerAction::GameEvent(String)` | Game-specific trigger responses |
| `TriggerAction::TweenPosition` | Direct door/platform tween in RON |
| `TriggerAction::DespawnSelf` | One-time pickup removal |
| `Hyperprism4D`, `Hypersphere4D` | Level geometry shapes |
| `CollisionFilter::trigger()` | Pickup/door/portal trigger zones |

---

## Phase 5: Polish (Game-Side)

**Depends on**: Engine Phase 5 complete (editor, lights, textures, input rebinding)
**Estimated effort**: 2-4 sessions

### 5.1 Game-Specific Editor Panels/Tools

**Source**: Agent P5 -- "Custom editor panels for weapon tuning, enemy config are game-side."

**Tasks**:
- [ ] Build weapon tuning panel (adjust damage, fire rate, spread in editor, hot-reload)
- [ ] Build enemy config panel (adjust health, speed, AI params, test spawn)
- [ ] Build level testing panel (start level from editor, toggle noclip, give all weapons)
- [ ] Use editor during development as dev-dependency:
  ```toml
  [dev-dependencies]
  rust4d_editor = { git = "https://github.com/Lemon9247/Rust4D.git" }
  ```

### 5.2 Input Rebinding UI / Pause Menu

**Source**: Agent P5 -- "Pause menu and rebinding UI are game-side."

**Tasks**:
- [ ] Implement pause menu (Escape key toggles):
  - Resume
  - Settings (leads to controls/audio/video submenus)
  - Restart Level
  - Quit to Main Menu
  - Quit to Desktop
- [ ] Implement key rebinding settings screen:
  - List all `InputAction` variants with current key bindings
  - Click action -> "Press a key..." -> captures next input -> calls `InputMap::rebind()`
  - Detect and warn about conflicts via `InputMap::conflicts()`
  - Reset to defaults button via `InputMap::reset_defaults()`
  - Persist via `InputMap::to_toml()` to config file
  - Load on startup via `InputMap::from_toml()`
- [ ] Audio settings: master/SFX/music/ambient volume sliders
- [ ] Video settings: resolution, fullscreen, V-sync (if engine supports)

### 5.3 Level Selection and Game State Management

**Tasks**:
- [ ] Implement main menu screen (Start Game, Level Select, Settings, Quit)
- [ ] Implement level selection screen (shows available levels with thumbnails/descriptions)
- [ ] Game state machine: MainMenu -> LevelSelect -> Playing -> Paused -> LevelComplete -> MainMenu
- [ ] Level completion screen (time, kills, secrets found)
- [ ] Save/load level completion state (which levels are unlocked)

### 5.4 Game-Specific Textures and Materials

**Source**: Agent P5 -- "Game-specific textures are loaded via TextureManager, assigned to Materials."

**Tasks**:
- [ ] Create/acquire wall, floor, ceiling texture assets (tileable)
- [ ] Assign textures to materials in RON scene files
- [ ] Ensure triplanar mapping looks good on level geometry
- [ ] Create distinct visual themes for different level areas / W-layers

### 5.5 Steam Integration Considerations

**Tasks** (future, post-MVP):
- [ ] Steam SDK integration (Steamworks crate)
- [ ] Achievements (first kill, level completions, secret areas)
- [ ] Leaderboards (level completion times)
- [ ] Cloud saves
- [ ] Steam Input API for controller support

### Engine APIs Used

| Engine API | Game Usage |
|---|---|
| `rust4d_editor` (dev-dependency) | Game-specific editor panels |
| `EditorHost` trait | Integrate editor into game for dev builds |
| `InputMap::rebind()` | Runtime key rebinding |
| `InputMap::to_toml()` / `from_toml()` | Persist key bindings |
| `TextureManager` | Load game texture assets |
| `PointLight4D` component | Place lights in levels |
| `OverlayRenderer` / egui | Pause menu, settings screens |

---

## W-Axis Gameplay Design Notes

**Source**: Cross-swarm synthesis, Agent P3 (W-specific enemy behaviors), Agent P4 (W-layered levels)

The W-axis is not a gimmick -- it fundamentally transforms every boomer shooter system. These design notes capture how the game should leverage 4D.

### W-Strafing as Dodge Technique

Players can dodge attacks by shifting along the W-axis. Hitscan attacks require W-alignment (attacker and target at same W), making W-strafing an effective dodge. This creates a natural skill curve: novice players fight in 3D, skilled players use W-strafing to avoid damage.

**Game implementation**: The W movement axis is already provided by `CharacterController4D`. The HUD W-position indicator is critical for players to track their W-position.

### Hyperspherical Explosions (R^4 Scaling)

In 4D, explosive blast volume scales as R^4 (vs R^3 in 3D). This means explosions are proportionally MORE powerful in 4D because they catch enemies in adjacent W-slices. This is a deliberate gameplay advantage for explosive weapons.

**Game implementation**: Use `PhysicsWorld::query_area_effect()` which naturally uses 4D Euclidean distance. Rockets are the counter to W-phasing enemies -- even if an enemy shifts W, a large enough explosion catches them.

### Hitscan W-Alignment

Hitscan weapons (shotgun) require precise W-alignment between shooter and target. A target at a different W-coordinate is effectively invisible and unhittable with hitscan. This creates a natural weapon hierarchy:
- Hitscan: high DPS, requires W-alignment (precision weapon)
- Projectile: medium DPS, some W-tolerance (tracking projectiles could follow in W)
- Explosive: lower single-target DPS, excellent W-coverage (area denial)

**Game implementation**: `PhysicsWorld::raycast()` naturally handles this -- rays in 4D only hit entities they geometrically intersect. An enemy at W=2.0 while the ray travels at W=0.0 is never hit.

### W-Layered Architecture for Levels

Levels exist across multiple W-layers. A single physical location (X,Y,Z) can have different rooms at different W values. W-portals transition the player between layers.

**Game implementation**: RON scene files place geometry at different W-coordinates. W-portals are trigger zones that tween the player's W-coordinate. The engine's rendering naturally shows/hides geometry based on W-distance from the current slice.

### W-Flanking Enemies

The W-Phaser enemy type approaches from adjacent W-slices, appearing as a ghostly shimmer before phasing in for an attack. This creates tension -- players hear audio cues and see faint sprites, knowing an attack is coming from "the 4th dimension."

**Game implementation**: Enemy AI controls the W-coordinate of the enemy's physics body. The engine's sprite W-distance fade handles the visual naturally. Audio system uses 4D spatial positioning so W-distance sounds are muffled/filtered.

### Cognitive Overload Risk and Mitigation

4D is genuinely hard to think about. The game must teach players incrementally:

1. **Level 1**: Standard 3D gameplay. No W-axis movement required. Players learn basic combat.
2. **Level 2**: Introduce W-indicator on HUD. Simple W-corridors (move forward in W to progress).
3. **Level 3**: Introduce W-strafing as dodge. Enemies telegraph attacks, giving time to W-dodge.
4. **Level 4**: Introduce W-phasing enemies. Players must learn to watch the W-indicator for incoming threats.
5. **Level 5+**: Full 4D gameplay with W-layered levels, multiple enemy types, all weapon types.

**Critical HUD element**: The W-position indicator must be prominent, intuitive, and always visible. It is the player's primary tool for understanding the 4th dimension.

---

## Session Estimates

| Phase | Game Sessions | Engine Prerequisite |
|---|---|---|
| Phase 0: Game Repo Setup | 1-2 | Engine split plan complete |
| Phase 1: Combat Core | 2-3 | Engine P1 (raycasting, events, triggers) |
| Phase 2: Weapons & Feedback | 3-4 | Engine P2 (audio, HUD, particles, screen effects) |
| Phase 3: Enemies | 3-4 | Engine P3 (sprites, FSM, spatial queries) |
| Phase 4: Level Design | 2-4 | Engine P4 (shapes, triggers, tweens, preview tool) |
| Phase 5: Polish | 2-4 | Engine P5 (editor, lights, textures, rebinding) |
| **Total Game Work** | **13-21** | |

### Parallelism

Game phases are sequential -- each builds on the previous. However, within each phase, multiple tasks can run in parallel:

```
Phase 0 (Setup)
  -> Phase 1 (Combat Core)
       Health/Damage system || Weapon system (after raycasting available)
    -> Phase 2 (Weapons & Feedback)
         HUD widgets || Sound integration || Particle presets (all independent)
      -> Phase 3 (Enemies)
           Enemy AI || Sprite assets || Spawn system (partially parallel)
        -> Phase 4 (Level Design)
             Level geometry || Door/elevator || Pickup system (partially parallel)
          -> Phase 5 (Polish)
               Menus || Editor panels || Textures || Steam (all independent)
```

---

## Dependencies on Engine

This table shows exactly which engine phases must be complete for each game phase to begin.

| Game Phase | Required Engine Work | Specific APIs Needed |
|---|---|---|
| Phase 0 | Split plan Phases 1-4 (ECS, rust4d_game, pluggable scenes, repo split) | hecs, CharacterController4D, InputMap, scene helpers |
| Phase 1 | Foundation + Engine P1 | `Ray4D`, `PhysicsWorld::raycast()`, `drain_collision_events()`, `CollisionEvent` with trigger enter/exit, `EventBus` |
| Phase 2 | Engine P2 | `AudioEngine4D`, `OverlayRenderer` (egui), `ParticleSystem`, `ScreenShake`, `TimedEffect` |
| Phase 3 | Engine P3 | `SpriteBatch`, `SpritePipeline`, `SpriteAnimation`, `StateMachine<S>`, `query_sphere()`, `query_area_effect()`, `line_of_sight()`, `apply_impulse()` |
| Phase 4 | Engine P4 | `Hyperprism4D`, `Hypersphere4D`, `TriggerDef`, `TweenManager`, `EasingFunction`, `TriggerAction::GameEvent` |
| Phase 5 | Engine P5 | `rust4d_editor` (dev-dep), `EditorHost`, `InputMap::rebind()`, `TextureManager`, `PointLight4D` |

### Critical Path

```
Engine Foundation (1-1.5 sessions)
  -> Engine Split Plan (9.5-14 sessions)
    -> Engine P1 (1.75 sessions) -> Game Phase 0 + 1
      -> Engine P2 (4.5-5.5 sessions) -> Game Phase 2
        -> Engine P3 (4 sessions) -> Game Phase 3
          -> Engine P4 (4.5 sessions) -> Game Phase 4
            -> Engine P5 (10-12.5 sessions) -> Game Phase 5
```

The engine critical path is approximately **31-39 sessions**. Game work can begin as soon as Engine P1 is done (after ~12-16 sessions of engine work), with each game phase unlocking as the corresponding engine phase completes.

---

## Summary

The Rust4D-Shooter game repo is responsible for all boomer-shooter-specific gameplay. It builds on the engine's generic 4D capabilities to create a unique game experience where the W-axis transforms every traditional FPS system. The total game-side effort is estimated at 13-21 sessions, running in parallel with engine development after the initial split is complete.

The W-axis is the game's defining feature. Every system -- weapons, enemies, levels, HUD -- must be designed with 4D in mind. The engine provides the mathematical and rendering foundation; the game makes it fun to play.
