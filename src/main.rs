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
use rust4d_input::CameraController;

/// Main application state
struct App {
    window: Option<Arc<Window>>,
    render_context: Option<RenderContext>,
    camera: Camera4D,
    controller: CameraController,
    last_frame: std::time::Instant,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            render_context: None,
            camera: Camera4D::new(),
            controller: CameraController::new(),
            last_frame: std::time::Instant::now(),
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

            self.window = Some(window);
            self.render_context = Some(render_context);
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
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    self.controller.process_keyboard(key, event.state);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.controller.process_mouse_button(button, state);
            }

            WindowEvent::RedrawRequested => {
                // Calculate delta time
                let now = std::time::Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                // Update camera
                let pos = self.controller.update(&mut self.camera, dt);

                // Log position occasionally (for debugging)
                if self.controller.is_moving() {
                    log::debug!("Camera position: ({:.2}, {:.2}, {:.2}, {:.2})",
                        pos.x, pos.y, pos.z, pos.w);
                }

                // Render
                if let Some(ctx) = &self.render_context {
                    // Dark blue background - will be replaced with actual rendering
                    let clear_color = wgpu::Color {
                        r: 0.02,
                        g: 0.02,
                        b: 0.08,
                        a: 1.0,
                    };

                    match ctx.render_clear(clear_color) {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            if let Some(ctx) = &mut self.render_context {
                                ctx.resize(ctx.size);
                            }
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            event_loop.exit();
                        }
                        Err(e) => log::warn!("Render error: {:?}", e),
                    }
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
