//! Rust4D - 4D Rendering Engine
//!
//! A 4D rendering engine that displays 3D cross-sections of 4D geometry.

mod config;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

use rust4d_core::{World, SceneManager};
use rust4d_render::{
    context::RenderContext,
    camera4d::Camera4D,
    pipeline::{SlicePipeline, RenderPipeline, SliceParams, RenderUniforms, perspective_matrix},
    RenderableGeometry, CheckerboardGeometry, position_gradient_color,
};
use rust4d_input::CameraController;
use rust4d_math::Vec4;

use config::AppConfig;

/// Main application state
struct App {
    /// Application configuration
    config: AppConfig,
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    /// Scene manager handling scene stack and physics
    scene_manager: SceneManager,
    /// Cached GPU geometry (rebuilt when world changes)
    geometry: RenderableGeometry,
    camera: Camera4D,
    controller: CameraController,
    last_frame: std::time::Instant,
    cursor_captured: bool,
}

impl App {
    fn new() -> Self {
        // Load configuration
        let config = AppConfig::load().unwrap_or_else(|e| {
            log::warn!("Failed to load config: {}. Using defaults.", e);
            AppConfig::default()
        });

        // Create scene manager and load scene from file
        // Pass physics config from TOML to the physics engine
        let mut scene_manager = SceneManager::new()
            .with_player_radius(config.scene.player_radius)
            .with_physics(config.physics.to_physics_config());

        // Load scene from configured path
        let scene_name = scene_manager.load_scene(&config.scene.path)
            .unwrap_or_else(|e| {
                panic!("Failed to load scene '{}': {}", config.scene.path, e);
            });

        // Instantiate and activate the scene
        scene_manager.instantiate(&scene_name)
            .unwrap_or_else(|e| panic!("Failed to instantiate scene: {}", e));
        scene_manager.push_scene(&scene_name)
            .unwrap_or_else(|e| panic!("Failed to push scene: {}", e));

        // Get player start from scene's player_spawn
        let player_start = scene_manager.active_scene()
            .and_then(|s| s.player_spawn)
            .map(|spawn| Vec4::new(spawn[0], spawn[1], spawn[2], spawn[3]))
            .unwrap_or_else(|| Vec4::new(
                config.camera.start_position[0],
                config.camera.start_position[1],
                config.camera.start_position[2],
                config.camera.start_position[3],
            ));

        // Build GPU geometry from the world
        let geometry = Self::build_geometry(scene_manager.active_world().unwrap());

        log::info!("Loaded scene '{}' with {} entities",
            scene_name,
            scene_manager.active_world().map(|w| w.entity_count()).unwrap_or(0));
        log::info!("Total geometry: {} vertices, {} tetrahedra",
            geometry.vertex_count(), geometry.tetrahedron_count());

        // Set camera to player start position
        let mut camera = Camera4D::new();
        camera.position = player_start;

        // Configure controller from config
        let controller = CameraController::new()
            .with_move_speed(config.input.move_speed)
            .with_w_move_speed(config.input.w_move_speed)
            .with_mouse_sensitivity(config.input.mouse_sensitivity)
            .with_smoothing_half_life(config.input.smoothing_half_life)
            .with_smoothing(config.input.smoothing_enabled);

        Self {
            config,
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            scene_manager,
            geometry,
            camera,
            controller,
            last_frame: std::time::Instant::now(),
            cursor_captured: false,
        }
    }

    /// Build GPU geometry from the world using custom coloring
    fn build_geometry(world: &World) -> RenderableGeometry {
        let mut geometry = RenderableGeometry::new();

        // Checkerboard pattern for the floor
        let checkerboard = CheckerboardGeometry::new(
            [0.3, 0.3, 0.35, 1.0], // Dark gray
            [0.7, 0.7, 0.75, 1.0], // Light gray
            2.0, // Cell size
        );

        for (_key, entity) in world.iter_with_keys() {
            if entity.has_tag("dynamic") {
                // Dynamic entities (tesseract): use position gradient
                geometry.add_entity_with_color(entity, &position_gradient_color);
            } else {
                // Static entities (floor): use checkerboard pattern
                geometry.add_entity_with_color(entity, &|v, _m| {
                    checkerboard.color_for_position(v.x, v.z)
                });
            }
        }

        geometry
    }

    /// Capture cursor for FPS-style controls
    fn capture_cursor(&mut self) {
        if let Some(window) = &self.window {
            // Try Locked mode first (best for FPS), fall back to Confined
            let grab_result = window.set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));

            if grab_result.is_ok() {
                window.set_cursor_visible(false);
                self.cursor_captured = true;
                log::info!("Cursor captured - Escape to release");
            } else {
                log::warn!("Failed to capture cursor");
            }
        }
    }

    /// Release cursor
    fn release_cursor(&mut self) {
        if let Some(window) = &self.window {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
            self.cursor_captured = false;
            log::info!("Cursor released - click to capture");
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title(&self.config.window.title)
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.config.window.width,
                    self.config.window.height,
                ));

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            // Create render context
            let render_context = pollster::block_on(RenderContext::new(window.clone()));

            // Create pipelines
            let mut slice_pipeline = SlicePipeline::new(&render_context.device);
            let mut render_pipeline = RenderPipeline::new(
                &render_context.device,
                render_context.config.format,
            );

            // Ensure depth texture exists
            render_pipeline.ensure_depth_texture(
                &render_context.device,
                render_context.size.width,
                render_context.size.height,
            );

            // Upload geometry
            slice_pipeline.upload_tetrahedra(
                &render_context.device,
                &self.geometry.vertices,
                &self.geometry.tetrahedra,
            );

            log::info!("Loaded {} vertices and {} tetrahedra",
                self.geometry.vertex_count(), self.geometry.tetrahedron_count());

            self.window = Some(window);
            self.render_context = Some(render_context);
            self.slice_pipeline = Some(slice_pipeline);
            self.render_pipeline = Some(render_pipeline);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                if let Some(ctx) = &mut self.render_context {
                    ctx.resize(physical_size);
                }
                if let (Some(ctx), Some(render_pipeline)) =
                    (&self.render_context, &mut self.render_pipeline)
                {
                    render_pipeline.ensure_depth_texture(
                        &ctx.device,
                        physical_size.width,
                        physical_size.height,
                    );
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    // Handle special keys on press
                    if event.state == ElementState::Pressed {
                        match key {
                            KeyCode::Escape => {
                                // Escape releases cursor first, then exits if pressed again
                                if self.cursor_captured {
                                    self.release_cursor();
                                } else {
                                    event_loop.exit();
                                }
                                return;
                            }
                            KeyCode::KeyR => {
                                self.camera.reset();
                                log::info!("Camera reset to starting position");
                            }
                            KeyCode::KeyF => {
                                if let Some(window) = &self.window {
                                    let new_fullscreen = if window.fullscreen().is_some() {
                                        None
                                    } else {
                                        Some(Fullscreen::Borderless(None))
                                    };
                                    window.set_fullscreen(new_fullscreen);
                                }
                            }
                            KeyCode::KeyG => {
                                let enabled = self.controller.toggle_smoothing();
                                log::info!("Input smoothing: {}", if enabled { "ON" } else { "OFF" });
                            }
                            _ => {}
                        }
                    }
                    // Pass to controller for movement keys
                    self.controller.process_keyboard(key, event.state);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                // Click to capture cursor (FPS style)
                if state == ElementState::Pressed && button == MouseButton::Left && !self.cursor_captured {
                    self.capture_cursor();
                }
                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Scroll wheel adjusts slice offset
                let scroll = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 100.0,
                };
                self.camera.adjust_slice_offset(scroll * 0.1);
            }

            WindowEvent::RedrawRequested => {
                // Calculate delta time
                let now = std::time::Instant::now();
                let raw_dt = (now - self.last_frame).as_secs_f32();
                // Cap dt to prevent huge physics steps on first frame or after window focus
                let dt = raw_dt.min(1.0 / 30.0); // Max 33ms per frame
                self.last_frame = now;

                // === PHYSICS-BASED GAME LOOP ===

                // 1. Get movement input from controller
                let (forward_input, right_input) = self.controller.get_movement_input();
                let w_input = self.controller.get_w_input();

                // 2. Calculate movement direction in world space using camera orientation
                // Get camera direction vectors
                let camera_forward = self.camera.forward();
                let camera_right = self.camera.right();
                let camera_ana = self.camera.ana();

                // Project all directions to XZW hyperplane (zero out Y for horizontal movement)
                // This ensures all movement rotates correctly in 4D space
                let forward_xzw = Vec4::new(camera_forward.x, 0.0, camera_forward.z, camera_forward.w).normalized();
                let right_xzw = Vec4::new(camera_right.x, 0.0, camera_right.z, camera_right.w).normalized();
                let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();

                // Combine movement direction (all axes rotate with 4D camera orientation)
                let move_dir = forward_xzw * forward_input + right_xzw * right_input
                    + ana_xzw * w_input;

                // 3. Apply movement to player via unified physics world (includes W for true 4D physics)
                let move_speed = self.controller.move_speed;
                if let Some(physics) = self.scene_manager.active_world_mut().and_then(|w| w.physics_mut()) {
                    physics.apply_player_movement(move_dir * move_speed);
                }

                // 4. Handle jump
                if self.controller.consume_jump() {
                    if let Some(physics) = self.scene_manager.active_world_mut().and_then(|w| w.physics_mut()) {
                        physics.player_jump();
                    }
                }

                // 5. Step world physics (tesseract + player dynamics) and sync entity transforms
                self.scene_manager.update(dt);

                // 6. Check for dirty entities and rebuild geometry if needed
                if self.scene_manager.active_world().map(|w| w.has_dirty_entities()).unwrap_or(false) {
                    // Rebuild geometry with new transforms
                    self.geometry = Self::build_geometry(self.scene_manager.active_world().unwrap());
                    // Re-upload to GPU
                    if let (Some(slice_pipeline), Some(ctx)) = (&mut self.slice_pipeline, &self.render_context) {
                        slice_pipeline.upload_tetrahedra(
                            &ctx.device,
                            &self.geometry.vertices,
                            &self.geometry.tetrahedra,
                        );
                    }
                    if let Some(w) = self.scene_manager.active_world_mut() {
                        w.clear_all_dirty();
                    }
                }

                // 7. Sync camera position to player physics (all 4 dimensions for true 4D physics)
                if let Some(pos) = self.scene_manager.active_world().and_then(|w| w.physics()).and_then(|p| p.player_position()) {
                    self.camera.position = pos;
                }

                // 8. Apply mouse look for camera rotation only
                // Note: controller.update() also applies movement which we don't want,
                // but we re-sync position below to discard the unwanted movement
                self.controller.update(&mut self.camera, dt, self.cursor_captured);

                // 9. Re-sync position after controller (discard its movement, keep rotation)
                if let Some(pos) = self.scene_manager.active_world().and_then(|w| w.physics()).and_then(|p| p.player_position()) {
                    self.camera.position = pos;
                }

                // Update window title with debug info
                if let Some(window) = &self.window {
                    let pos = self.camera.position;
                    let base_title = &self.config.window.title;
                    let title = if self.cursor_captured {
                        format!(
                            "{} - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Esc to release]",
                            base_title, pos.x, pos.y, pos.z, pos.w, self.camera.get_slice_w()
                        )
                    } else {
                        format!(
                            "{} - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Click to capture]",
                            base_title, pos.x, pos.y, pos.z, pos.w, self.camera.get_slice_w()
                        )
                    };
                    window.set_title(&title);
                }

                // Render
                if let (Some(ctx), Some(slice_pipeline), Some(render_pipeline)) = (
                    &self.render_context,
                    &self.slice_pipeline,
                    &self.render_pipeline,
                ) {
                    // Camera positions
                    let pos = self.camera.position;
                    let eye_3d = [pos.x, pos.y, pos.z];
                    let camera_pos_4d = [pos.x, pos.y, pos.z, pos.w];

                    // Update slice parameters
                    let camera_matrix = self.camera.rotation_matrix();
                    let slice_params = SliceParams {
                        slice_w: self.camera.get_slice_w(),
                        tetrahedron_count: self.geometry.tetrahedron_count() as u32,
                        _padding: [0.0; 2],
                        camera_matrix,
                        camera_eye: eye_3d,
                        _padding2: 0.0,
                        camera_position: camera_pos_4d,
                    };
                    slice_pipeline.update_params(&ctx.queue, &slice_params);

                    // Create view and projection matrices
                    let aspect = ctx.aspect_ratio();
                    let proj_matrix = perspective_matrix(
                        self.config.camera.fov.to_radians(),
                        aspect,
                        self.config.camera.near,
                        self.config.camera.far,
                    );

                    // The slice shader transforms 4D geometry to camera space:
                    // 1. Translates by -camera_position (camera at origin)
                    // 2. Rotates by inverse(camera_matrix) to align with camera view
                    // So the output 3D coordinates are already in camera space.
                    // View matrix is identity - no additional transformation needed.
                    let view_matrix = [
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, 1.0, 0.0, 0.0],
                        [0.0, 0.0, 1.0, 0.0],
                        [0.0, 0.0, 0.0, 1.0],
                    ];

                    let render_uniforms = RenderUniforms {
                        view_matrix,
                        projection_matrix: proj_matrix,
                        light_dir: self.config.rendering.light_dir,
                        _padding: 0.0,
                        ambient_strength: self.config.rendering.ambient_strength,
                        diffuse_strength: self.config.rendering.diffuse_strength,
                        w_color_strength: 0.5,
                        w_range: 2.0,
                    };
                    render_pipeline.update_uniforms(&ctx.queue, &render_uniforms);

                    // Get surface texture
                    let output = match ctx.surface.get_current_texture() {
                        Ok(output) => output,
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(ctx) = &mut self.render_context {
                                ctx.resize(ctx.size);
                            }
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                            return;
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            event_loop.exit();
                            return;
                        }
                        Err(e) => {
                            log::warn!("Surface error: {:?}", e);
                            return;
                        }
                    };

                    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

                    // Create command encoder
                    let mut encoder = ctx.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        },
                    );

                    // Reset counter and run compute pass
                    slice_pipeline.reset_counter(&ctx.queue);
                    slice_pipeline.run_slice_pass(&mut encoder);

                    // Copy triangle count to indirect buffer
                    render_pipeline.prepare_indirect_draw(&mut encoder, slice_pipeline.counter_buffer());

                    // Render pass
                    let bg = &self.config.rendering.background_color;
                    render_pipeline.render(
                        &mut encoder,
                        &view,
                        slice_pipeline.output_buffer(),
                        wgpu::Color {
                            r: bg[0] as f64,
                            g: bg[1] as f64,
                            b: bg[2] as f64,
                            a: bg[3] as f64,
                        },
                    );

                    // Submit
                    ctx.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }

                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.controller.process_mouse_motion(delta.0, delta.1);
        }
    }
}

fn main() {
    // Initialize logging
    env_logger::init();
    log::info!("Starting Rust4D");

    // Create event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Create and run application
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");
}

#[cfg(test)]
mod integration_tests {
    use super::config::AppConfig;

    #[test]
    fn test_env_override() {
        std::env::set_var("R4D_WINDOW__TITLE", "Test From Env");
        let config = AppConfig::load().unwrap();
        println!("Window title: {}", config.window.title);
        assert_eq!(config.window.title, "Test From Env");
    }

    #[test]
    fn test_user_config_loading() {
        // Remove env var to test file-based config
        std::env::remove_var("R4D_WINDOW__TITLE");

        // Debug: print current dir and check if files exist
        let cwd = std::env::current_dir().unwrap();
        println!("Current dir: {:?}", cwd);
        println!("config/default.toml exists: {}", cwd.join("config/default.toml").exists());
        println!("config/user.toml exists: {}", cwd.join("config/user.toml").exists());

        let config = AppConfig::load().unwrap();
        println!("Window title from file: {}", config.window.title);
    }
}
