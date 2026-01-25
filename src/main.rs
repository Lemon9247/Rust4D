//! Rust4D - 4D Rendering Engine
//!
//! A 4D rendering engine that displays 3D cross-sections of 4D geometry.

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use rust4d_render::context::RenderContext;
use rust4d_render::camera4d::Camera4D;
use rust4d_render::geometry::Tesseract;
use rust4d_render::pipeline::{
    SlicePipeline, RenderPipeline, SliceParams, RenderUniforms,
    Simplex4D, Vertex4D, perspective_matrix, look_at_matrix,
};
use rust4d_input::CameraController;

/// Main application state
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    slice_pipeline: Option<SlicePipeline>,
    render_pipeline: Option<RenderPipeline>,
    simplices: Vec<Simplex4D>,
    camera: Camera4D,
    controller: CameraController,
    last_frame: std::time::Instant,
}

impl App {
    fn new() -> Self {
        // Create tesseract geometry
        let tesseract = Tesseract::new(2.0);
        let simplices = Self::tesseract_to_simplices(&tesseract);

        Self {
            window: None,
            render_context: None,
            slice_pipeline: None,
            render_pipeline: None,
            simplices,
            camera: Camera4D::new(),
            controller: CameraController::new(),
            last_frame: std::time::Instant::now(),
        }
    }

    /// Convert tesseract geometry to GPU simplices
    fn tesseract_to_simplices(tesseract: &Tesseract) -> Vec<Simplex4D> {
        // Colors for the tesseract - gradient based on position
        let colors = [
            [1.0, 0.3, 0.3, 1.0], // Red
            [0.3, 1.0, 0.3, 1.0], // Green
            [0.3, 0.3, 1.0, 1.0], // Blue
            [1.0, 1.0, 0.3, 1.0], // Yellow
            [1.0, 0.3, 1.0, 1.0], // Magenta
            [0.3, 1.0, 1.0, 1.0], // Cyan
            [1.0, 0.6, 0.3, 1.0], // Orange
            [0.6, 0.3, 1.0, 1.0], // Purple
        ];

        tesseract.simplices.iter().enumerate().map(|(simplex_idx, indices)| {
            let base_color = colors[simplex_idx % colors.len()];
            let vertices: [Vertex4D; 5] = std::array::from_fn(|i| {
                let v = tesseract.vertices[indices[i]];
                Vertex4D {
                    position: [v.x, v.y, v.z, v.w],
                    color: base_color,
                }
            });
            Simplex4D { vertices }
        }).collect()
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

            // Upload tesseract simplices
            slice_pipeline.upload_simplices(&render_context.device, &self.simplices);

            log::info!("Loaded {} simplices from tesseract", self.simplices.len());

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
                    self.controller.process_keyboard(key, event.state);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
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
                let _pos = self.controller.update(&mut self.camera, dt);

                // Render
                if let (Some(ctx), Some(slice_pipeline), Some(render_pipeline)) = (
                    &self.render_context,
                    &self.slice_pipeline,
                    &self.render_pipeline,
                ) {
                    // Update slice parameters
                    let camera_matrix = self.camera.rotation_matrix();
                    let slice_params = SliceParams {
                        slice_w: self.camera.get_slice_w(),
                        _padding: [0.0; 3],
                        camera_matrix,
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

                    // Camera position in 3D (use xyz of 4D position)
                    let eye = [self.camera.position.x, self.camera.position.y, self.camera.position.z];
                    let forward = self.camera.forward();
                    let target = [
                        eye[0] + forward.x,
                        eye[1] + forward.y,
                        eye[2] + forward.z,
                    ];
                    let view_matrix = look_at_matrix(eye, target, [0.0, 1.0, 0.0]);
                    let view_proj = rust4d_render::pipeline::mat4_mul(proj_matrix, view_matrix);

                    let render_uniforms = RenderUniforms {
                        view_proj,
                        camera_pos: eye,
                        _padding: 0.0,
                        light_dir: [0.5, 1.0, 0.3],
                        _padding2: 0.0,
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
