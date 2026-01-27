# Phase 8: main.rs Decomposition

**Status:** Not Started
**Sessions:** 2-3
**Dependencies:** All previous phases
**Parallelizable With:** None (final integration)

---

## Goal

Decompose the 500+ line `App` struct into smaller, testable systems. This is the final phase that brings together all previous improvements.

---

## Problem

Current `main.rs` is a monolithic "god struct":

```rust
struct App {
    // Window management
    window: Arc<Window>,

    // Rendering
    context: RenderContext,
    slice_pipeline: SlicePipeline,
    render_pipeline: RenderPipeline,
    geometry: RenderableGeometry,

    // World and entities
    world: World,
    tesseract_entity: EntityKey,

    // Camera
    camera: Camera4D,
    controller: CameraController,

    // Physics (should be in World)
    player_body: BodyKey,

    // State
    last_frame: Instant,
    mouse_locked: bool,
    clear_color: wgpu::Color,
}
```

~490 lines mixing window management, rendering, physics, input, and game logic.

---

## Target Architecture

```rust
// main.rs - ~100 lines
fn main() {
    let mut engine = GameEngine::new();
    engine.run();
}

// game_engine.rs
pub struct GameEngine {
    window_system: WindowSystem,
    input_system: InputSystem,
    physics_system: PhysicsSystem,
    render_system: RenderSystem,
    world: World,
}

impl GameEngine {
    pub fn run(&mut self) {
        // Event loop
        self.window_system.run(|event, control_flow| {
            match event {
                WindowEvent::RedrawRequested => {
                    let dt = self.input_system.update();
                    self.physics_system.update(&mut self.world, dt);
                    self.render_system.render(&self.world);
                }
                // ...
            }
        });
    }
}
```

---

## New Files to Create

| File | Purpose |
|------|---------|
| `src/engine/mod.rs` | GameEngine struct and run loop |
| `src/engine/window_system.rs` | Window management |
| `src/engine/input_system.rs` | Input handling and camera control |
| `src/engine/render_system.rs` | Rendering orchestration |
| `src/scene/mod.rs` | Scene loading and setup |
| `src/scene/scene_builder.rs` | Fluent scene construction |

---

## Implementation Steps

### Wave 1: Create Engine Module (Sequential)

1. Create directory structure:
   ```
   src/
   ├── main.rs           # Entry point only
   ├── engine/
   │   ├── mod.rs
   │   ├── window_system.rs
   │   ├── input_system.rs
   │   └── render_system.rs
   └── scene/
       ├── mod.rs
       └── scene_builder.rs
   ```

2. Create `src/engine/mod.rs`:
   ```rust
   mod window_system;
   mod input_system;
   mod render_system;

   pub use window_system::WindowSystem;
   pub use input_system::InputSystem;
   pub use render_system::RenderSystem;

   use rust4d_core::World;
   use winit::event_loop::EventLoop;

   pub struct GameEngine {
       event_loop: Option<EventLoop<()>>,
       window_system: WindowSystem,
       input_system: InputSystem,
       render_system: RenderSystem,
       world: World,
   }

   impl GameEngine {
       pub fn new() -> Self {
           let event_loop = EventLoop::new().unwrap();
           let window_system = WindowSystem::new(&event_loop);
           let input_system = InputSystem::new();
           let render_system = RenderSystem::new(&window_system);
           let world = World::new();

           Self {
               event_loop: Some(event_loop),
               window_system,
               input_system,
               render_system,
               world,
           }
       }

       pub fn world_mut(&mut self) -> &mut World {
           &mut self.world
       }

       pub fn run(mut self) {
           let event_loop = self.event_loop.take().unwrap();
           event_loop.run(move |event, elwt| {
               self.handle_event(event, elwt);
           }).unwrap();
       }

       fn handle_event(&mut self, event: Event<()>, elwt: &EventLoopWindowTarget<()>) {
           // Delegate to appropriate system
           self.input_system.handle_event(&event);
           self.window_system.handle_event(&event, elwt);

           match event {
               Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                   self.update();
               }
               _ => {}
           }
       }

       fn update(&mut self) {
           let dt = self.input_system.delta_time();

           // Physics
           self.input_system.apply_to_world(&mut self.world);
           self.world.update(dt);

           // Render
           self.render_system.render(&self.world, self.input_system.camera());
       }
   }
   ```

### Wave 2: Window System (Sequential)

1. Create `src/engine/window_system.rs`:
   ```rust
   use std::sync::Arc;
   use winit::window::Window;
   use winit::event_loop::EventLoop;

   pub struct WindowSystem {
       window: Arc<Window>,
       size: (u32, u32),
   }

   impl WindowSystem {
       pub fn new(event_loop: &EventLoop<()>) -> Self {
           let window = Arc::new(
               Window::builder()
                   .with_title("Rust4D")
                   .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
                   .build(event_loop)
                   .unwrap()
           );

           let size = window.inner_size();

           Self {
               window,
               size: (size.width, size.height),
           }
       }

       pub fn window(&self) -> &Arc<Window> {
           &self.window
       }

       pub fn size(&self) -> (u32, u32) {
           self.size
       }

       pub fn handle_event(&mut self, event: &Event<()>, elwt: &EventLoopWindowTarget<()>) {
           match event {
               Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                   self.size = (size.width, size.height);
               }
               Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                   elwt.exit();
               }
               _ => {}
           }
       }

       pub fn request_redraw(&self) {
           self.window.request_redraw();
       }
   }
   ```

### Wave 3: Input System (Sequential)

1. Create `src/engine/input_system.rs`:
   ```rust
   use std::time::Instant;
   use rust4d_input::CameraController;
   use rust4d_render::Camera4D;
   use rust4d_core::World;
   use rust4d_math::Vec4;

   pub struct InputSystem {
       controller: CameraController,
       camera: Camera4D,
       last_frame: Instant,
       dt: f32,
       mouse_locked: bool,
   }

   impl InputSystem {
       pub fn new() -> Self {
           Self {
               controller: CameraController::new(0.003, 3.0),
               camera: Camera4D::new(),
               last_frame: Instant::now(),
               dt: 0.0,
               mouse_locked: false,
           }
       }

       pub fn handle_event(&mut self, event: &Event<()>) {
           // Process keyboard/mouse input
           self.controller.process_event(event);

           match event {
               Event::WindowEvent { event: WindowEvent::MouseInput { button: MouseButton::Left, state: ElementState::Pressed, .. }, .. } => {
                   self.mouse_locked = true;
               }
               Event::WindowEvent { event: WindowEvent::KeyboardInput { event, .. }, .. } => {
                   if event.physical_key == KeyCode::Escape && event.state == ElementState::Pressed {
                       self.mouse_locked = false;
                   }
               }
               _ => {}
           }
       }

       pub fn update_delta_time(&mut self) {
           let now = Instant::now();
           self.dt = now.duration_since(self.last_frame).as_secs_f32();
           self.last_frame = now;
       }

       pub fn delta_time(&self) -> f32 {
           self.dt
       }

       pub fn camera(&self) -> &Camera4D {
           &self.camera
       }

       pub fn camera_mut(&mut self) -> &mut Camera4D {
           &mut self.camera
       }

       pub fn apply_to_world(&mut self, world: &mut World) {
           let (forward, right) = self.controller.get_movement_input();

           // Get camera-relative movement direction
           let camera_forward = self.camera.forward().with_y(0.0).normalized();
           let camera_right = self.camera.right().with_y(0.0).normalized();
           let move_dir = camera_forward * forward + camera_right * right;

           if let Some(physics) = world.physics_mut() {
               physics.apply_player_movement(move_dir, 5.0);

               if self.controller.consume_jump() {
                   physics.player_jump(8.0);
               }
           }

           // Update camera rotation
           self.controller.apply_to_camera(&mut self.camera, self.dt);

           // Sync camera to player position
           if let Some(physics) = world.physics() {
               if let Some(pos) = physics.player_position() {
                   self.camera.set_position(pos);
               }
           }
       }

       pub fn is_mouse_locked(&self) -> bool {
           self.mouse_locked
       }
   }
   ```

### Wave 4: Render System (Can parallelize with Wave 3)

1. Create `src/engine/render_system.rs`:
   ```rust
   use rust4d_core::World;
   use rust4d_render::{RenderContext, SlicePipeline, RenderPipeline, RenderableGeometry, Camera4D};

   pub struct RenderSystem {
       context: RenderContext,
       slice_pipeline: SlicePipeline,
       render_pipeline: RenderPipeline,
       geometry: RenderableGeometry,
       clear_color: wgpu::Color,
   }

   impl RenderSystem {
       pub fn new(window_system: &WindowSystem) -> Self {
           let context = RenderContext::new(window_system.window()).await;
           let slice_pipeline = SlicePipeline::new(&context);
           let render_pipeline = RenderPipeline::new(&context);

           Self {
               context,
               slice_pipeline,
               render_pipeline,
               geometry: RenderableGeometry::new(),
               clear_color: wgpu::Color { r: 0.1, g: 0.1, b: 0.15, a: 1.0 },
           }
       }

       pub fn resize(&mut self, width: u32, height: u32) {
           self.context.resize(width, height);
       }

       pub fn update_geometry(&mut self, world: &World) {
           if world.has_dirty_entities() {
               for (key, entity) in world.dirty_entities() {
                   let color_fn = Self::get_color_fn(entity);
                   self.geometry.update_entity(key, entity, &color_fn);
               }

               // Upload to GPU
               self.slice_pipeline.upload_vertices(&self.geometry.combined_vertices());
               self.slice_pipeline.upload_tetrahedra(&self.geometry.combined_tetrahedra());
           }
       }

       pub fn render(&mut self, world: &World, camera: &Camera4D) {
           self.update_geometry(world);

           let output = self.context.surface().get_current_texture().unwrap();
           let view = output.texture.create_view(&Default::default());

           let mut encoder = self.context.device().create_command_encoder(&Default::default());

           // Slice pass
           self.slice_pipeline.render(&mut encoder, camera);

           // Final render pass
           self.render_pipeline.render(&mut encoder, &view, &self.slice_pipeline, self.clear_color);

           self.context.queue().submit([encoder.finish()]);
           output.present();
       }

       fn get_color_fn(entity: &Entity) -> impl Fn(Vec4, Material) -> Vec4 {
           if entity.tags.contains("dynamic") {
               |pos, _mat| {
                   // Position gradient
                   Vec4::new(
                       (pos.x + 2.0) / 4.0,
                       (pos.y + 2.0) / 4.0,
                       (pos.z + 2.0) / 4.0,
                       1.0,
                   )
               }
           } else {
               |pos, _mat| {
                   // Checkerboard
                   let checker = ((pos.x * 2.0).floor() + (pos.z * 2.0).floor()) as i32 % 2;
                   if checker == 0 {
                       Vec4::new(0.3, 0.3, 0.3, 1.0)
                   } else {
                       Vec4::new(0.7, 0.7, 0.7, 1.0)
                   }
               }
           }
       }
   }
   ```

### Wave 5: Scene Builder (Sequential)

1. Create `src/scene/scene_builder.rs`:
   ```rust
   use rust4d_core::{World, Entity, EntityKey};
   use rust4d_physics::{PhysicsConfig, RigidBody4D, StaticCollider, PhysicsMaterial, CollisionFilter, BodyType};
   use rust4d_math::{Vec4, Tesseract4D, Hyperplane4D};

   pub struct SceneBuilder {
       world: World,
       player_key: Option<EntityKey>,
   }

   impl SceneBuilder {
       pub fn new() -> Self {
           Self {
               world: World::new(),
               player_key: None,
           }
       }

       pub fn with_physics(mut self, gravity: f32) -> Self {
           self.world = self.world.with_physics(PhysicsConfig::new(gravity));
           self
       }

       pub fn add_floor(mut self, y: f32, size: f32, material: PhysicsMaterial) -> Self {
           // Add physics floor
           if let Some(physics) = self.world.physics_mut() {
               physics.add_static_collider(StaticCollider::floor(y, material));
           }

           // Add visual floor entity
           let floor_shape = Hyperplane4D::new(y, size, 10, size / 2.0, 0.001);
           self.world.add_entity(
               Entity::new(Box::new(floor_shape))
                   .with_name("floor")
                   .with_tag("static")
           );

           self
       }

       pub fn add_tesseract(mut self, position: Vec4, size: f32, name: &str) -> Self {
           let shape = Tesseract4D::new(size);
           let half_extents = Vec4::splat(size / 2.0);

           // Add physics body
           let body = RigidBody4D::new_aabb(position, half_extents)
               .with_body_type(BodyType::Dynamic)
               .with_material(PhysicsMaterial::WOOD);

           let body_key = self.world.physics_mut().unwrap().add_body(body);

           // Add entity
           self.world.add_entity(
               Entity::new(Box::new(shape))
                   .with_name(name)
                   .with_tag("dynamic")
                   .with_physics_body(body_key)
           );

           self
       }

       pub fn add_player(mut self, position: Vec4, radius: f32) -> Self {
           let body = RigidBody4D::new_sphere(position, radius)
               .with_body_type(BodyType::Kinematic)
               .with_filter(CollisionFilter::player());

           let body_key = self.world.physics_mut().unwrap().add_body(body);
           self.world.physics_mut().unwrap().set_player_body(body_key);

           self
       }

       pub fn build(self) -> World {
           self.world
       }
   }
   ```

### Wave 6: Simplified main.rs (Sequential)

1. Rewrite `src/main.rs`:
   ```rust
   mod engine;
   mod scene;

   use engine::GameEngine;
   use scene::SceneBuilder;
   use rust4d_physics::PhysicsMaterial;
   use rust4d_math::Vec4;

   fn main() {
       let mut engine = GameEngine::new();

       // Build scene
       let world = SceneBuilder::new()
           .with_physics(-20.0)
           .add_floor(-2.0, 10.0, PhysicsMaterial::CONCRETE)
           .add_player(Vec4::new(0.0, 0.0, 5.0, 0.0), 0.5)
           .add_tesseract(Vec4::new(0.0, 0.0, 0.0, 0.0), 2.0, "tesseract")
           .build();

       *engine.world_mut() = world;

       engine.run();
   }
   ```

---

## Commits

1. "Create engine module structure"
2. "Extract WindowSystem from App"
3. "Extract InputSystem from App"
4. "Extract RenderSystem from App"
5. "Add SceneBuilder for declarative scene setup"
6. "Simplify main.rs to ~30 lines"

---

## Verification

1. **Unit tests for each system:**
   ```rust
   #[test]
   fn test_scene_builder() {
       let world = SceneBuilder::new()
           .with_physics(-10.0)
           .add_floor(0.0, 10.0, PhysicsMaterial::CONCRETE)
           .add_player(Vec4::ZERO, 0.5)
           .build();

       assert!(world.physics().is_some());
       assert!(world.get_by_name("floor").is_some());
   }

   #[test]
   fn test_input_system_delta_time() {
       let mut input = InputSystem::new();
       std::thread::sleep(std::time::Duration::from_millis(16));
       input.update_delta_time();
       assert!(input.delta_time() > 0.01);
   }
   ```

2. **Integration test:**
   - Run the game
   - All existing functionality works
   - Same visual output

3. **Code metrics:**
   - `main.rs` under 50 lines
   - Each system under 200 lines
   - No system has more than 3 dependencies

---

## Future Extensions

After decomposition, adding new features is easier:
- **Audio system:** Add `AudioSystem` to engine
- **UI system:** Add `UiSystem` for menus, HUD
- **Save/Load:** SceneBuilder can serialize to/from files
- **Editor mode:** Systems can be paused/inspected individually

---

## Rollback Plan

Keep the old `App` struct in a separate branch. The decomposition is structural, not functional—behavior should be identical.
