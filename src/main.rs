//! Rust4D - 4D Rendering Engine
//!
//! A 4D rendering engine that displays 3D cross-sections of 4D geometry.

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

use rust4d_render::context::RenderContext;
use rust4d_render::camera4d::Camera4D;
use rust4d_render::geometry::{Tesseract, Hyperplane};
use rust4d_render::pipeline::{
    SlicePipeline, RenderPipeline, SliceParams, RenderUniforms,
    Vertex4D, GpuTetrahedron, perspective_matrix, look_at_matrix,
};
use rust4d_input::CameraController;

/// Main application state
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    vertices: Vec<Vertex4D>,
    tetrahedra: Vec<GpuTetrahedron>,
    camera: Camera4D,
    controller: CameraController,
    last_frame: std::time::Instant,
    cursor_captured: bool,
}

impl App {
    fn new() -> Self {
        // Create tesseract geometry
        let mut tesseract = Tesseract::new(2.0);
        let (mut vertices, mut tetrahedra) = Self::tesseract_to_tetrahedra(&mut tesseract);

        // Create checkerboard hyperplane below the tesseract
        // y=-2.0 (below tesseract), size=10 (extends from -10 to +10 in XZ),
        // 10x10 grid, cell_size=2.0 for checkerboard, w_extent=5.0, tiny thickness
        let hyperplane = Hyperplane::new(-2.0, 10.0, 10, 2.0, 5.0, 0.001);
        let (plane_vertices, plane_tetrahedra) = Self::hyperplane_to_tetrahedra(&hyperplane, vertices.len());

        // Combine geometry
        vertices.extend(plane_vertices);
        tetrahedra.extend(plane_tetrahedra);

        log::info!("Total geometry: {} vertices, {} tetrahedra", vertices.len(), tetrahedra.len());

        Self {
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            vertices,
            tetrahedra,
            camera: Camera4D::new(),
            controller: CameraController::new(),
            last_frame: std::time::Instant::now(),
            cursor_captured: false,
        }
    }

    /// Convert tesseract geometry to GPU vertices and tetrahedra
    fn tesseract_to_tetrahedra(tesseract: &mut Tesseract) -> (Vec<Vertex4D>, Vec<GpuTetrahedron>) {
        // Convert tesseract vertices to GPU format with colors
        let vertices: Vec<Vertex4D> = tesseract.vertices.iter().enumerate().map(|(_i, v)| {
            // Color based on vertex position - creates visual gradient
            let color = [
                (v.x + 1.0) / 2.0, // Red from x
                (v.y + 1.0) / 2.0, // Green from y
                (v.z + 1.0) / 2.0, // Blue from z
                1.0,
            ];
            Vertex4D::new([v.x, v.y, v.z, v.w], color)
        }).collect();

        // Get tetrahedra decomposition
        let tetrahedra: Vec<GpuTetrahedron> = tesseract.tetrahedra().iter().map(|tet| {
            GpuTetrahedron::from_indices([
                tet.vertices[0] as u32,
                tet.vertices[1] as u32,
                tet.vertices[2] as u32,
                tet.vertices[3] as u32,
            ])
        }).collect();

        log::info!("Generated {} vertices and {} tetrahedra from tesseract",
            vertices.len(), tetrahedra.len());

        (vertices, tetrahedra)
    }

    /// Convert hyperplane geometry to GPU vertices and tetrahedra
    fn hyperplane_to_tetrahedra(hyperplane: &Hyperplane, vertex_offset: usize) -> (Vec<Vertex4D>, Vec<GpuTetrahedron>) {
        // Convert hyperplane vertices to GPU format with pre-computed colors
        let vertices: Vec<Vertex4D> = hyperplane.vertices.iter()
            .zip(hyperplane.colors.iter())
            .map(|(v, color)| {
                Vertex4D::new([v.x, v.y, v.z, v.w], *color)
            })
            .collect();

        // Convert tetrahedra with offset for combined vertex buffer
        let tetrahedra: Vec<GpuTetrahedron> = hyperplane.tetrahedra.iter().map(|tet| {
            GpuTetrahedron::from_indices([
                (tet.vertices[0] + vertex_offset) as u32,
                (tet.vertices[1] + vertex_offset) as u32,
                (tet.vertices[2] + vertex_offset) as u32,
                (tet.vertices[3] + vertex_offset) as u32,
            ])
        }).collect();

        log::info!("Generated {} vertices and {} tetrahedra from hyperplane",
            vertices.len(), tetrahedra.len());

        (vertices, tetrahedra)
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
                .with_title("Rust4D - 4D Rendering Engine")
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));

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

            // Upload tesseract geometry (tetrahedra mode)
            slice_pipeline.upload_tetrahedra(&render_context.device, &self.vertices, &self.tetrahedra);

            log::info!("Loaded {} vertices and {} tetrahedra from tesseract",
                self.vertices.len(), self.tetrahedra.len());

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
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Update camera
                let _pos = self.controller.update(&mut self.camera, dt, self.cursor_captured);

                // Update window title with debug info
                if let Some(window) = &self.window {
                    let pos = self.camera.position;
                    let title = if self.cursor_captured {
                        format!(
                            "Rust4D - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Esc to release]",
                            pos.x, pos.y, pos.z, pos.w, self.camera.get_slice_w()
                        )
                    } else {
                        format!(
                            "Rust4D - ({:.1}, {:.1}, {:.1}, {:.1}) W:{:.2} [Click to capture]",
                            pos.x, pos.y, pos.z, pos.w, self.camera.get_slice_w()
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
                    // Camera position in 3D (use xyz of 4D position)
                    let eye = [self.camera.position.x, self.camera.position.y, self.camera.position.z];

                    // Update slice parameters
                    let camera_matrix = self.camera.rotation_matrix();
                    let slice_params = SliceParams {
                        slice_w: self.camera.get_slice_w(),
                        tetrahedron_count: self.tetrahedra.len() as u32,
                        _padding: [0.0; 2],
                        camera_matrix,
                        camera_eye: eye,
                        _padding2: 0.0,
                    };
                    slice_pipeline.update_params(&ctx.queue, &slice_params);

                    // Create view and projection matrices
                    let aspect = ctx.aspect_ratio();
                    let proj_matrix = perspective_matrix(
                        std::f32::consts::FRAC_PI_4,
                        aspect,
                        0.1,
                        100.0,
                    );
                    let forward = self.camera.forward();
                    let up = self.camera.up();
                    let target = [
                        eye[0] + forward.x,
                        eye[1] + forward.y,
                        eye[2] + forward.z,
                    ];
                    // Use camera's actual up vector to avoid gimbal lock distortion
                    let view_matrix = look_at_matrix(eye, target, [up.x, up.y, up.z]);

                    let render_uniforms = RenderUniforms {
                        view_matrix,
                        projection_matrix: proj_matrix,
                        light_dir: [0.5, 1.0, 0.3],
                        _padding: 0.0,
                        ambient_strength: 0.3,
                        diffuse_strength: 0.7,
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
                    render_pipeline.render(
                        &mut encoder,
                        &view,
                        slice_pipeline.output_buffer(),
                        wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.08,
                            a: 1.0,
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
