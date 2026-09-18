use crate::hud::{FloatingToolbar, ToolbarAction};
use crate::state_machine::OverlayState;
use crate::surface::OverlaySurface;
use camerashot_core::geometry::Point;
use std::num::NonZeroU32;
use std::rc::Rc;
use tiny_skia::Pixmap;
use tracing::{error, info};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

/// The overlay application that owns the window, softbuffer surface, and overlay state.
pub struct OverlayApp {
    pub surface: OverlaySurface,
    window: Option<Rc<Window>>,
    sb_surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    scratch: Pixmap,
    modifiers: ModifiersState,
    /// Last known cursor position in physical pixels.
    last_cursor: Point,
    /// 0 = cancelled/Esc, 1 = copy success, 2 = copy failure
    pub exit_code: i32,
}

impl OverlayApp {
    pub fn new(surface: OverlaySurface) -> Self {
        let w = surface.width;
        let h = surface.height;
        let scratch = Pixmap::new(w, h).expect("Failed to create scratch pixmap");

        Self {
            surface,
            window: None,
            sb_surface: None,
            scratch,
            modifiers: ModifiersState::empty(),
            last_cursor: Point::ZERO,
            exit_code: 0,
        }
    }

    fn copy_and_exit(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(pixmap) = self.surface.export_selection_pixmap() {
            let w = pixmap.width();
            let h = pixmap.height();
            info!("Copying selection to clipboard: {}×{}", w, h);
            match camerashot_platform::clipboard::copy_rgba_image(w, h, pixmap.data()) {
                Ok(()) => {
                    info!("Copied to clipboard successfully");
                    self.exit_code = 0;
                }
                Err(e) => {
                    error!("Clipboard copy failed: {}", e);
                    self.exit_code = 2;
                }
            }
        } else {
            error!("No selection to copy");
            self.exit_code = 2;
        }
        event_loop.exit();
    }

    fn present_frame(&mut self) {
        // Render the overlay frame into scratch
        self.surface.render_frame(&mut self.scratch);

        if let Some(ref mut sb_surface) = self.sb_surface {
            let w = self.surface.width;
            let h = self.surface.height;
            let Ok(mut buffer) = sb_surface.buffer_mut() else {
                return;
            };

            // Convert RGBA → 0x00RRGGBB for softbuffer
            let scratch_data = self.scratch.data();
            let pixel_count = (w as usize) * (h as usize);
            for i in 0..pixel_count {
                let idx = i * 4;
                let r = scratch_data[idx] as u32;
                let g = scratch_data[idx + 1] as u32;
                let b = scratch_data[idx + 2] as u32;
                buffer[i] = (r << 16) | (g << 8) | b;
            }

            let _ = buffer.present();
        }
    }
}

impl ApplicationHandler for OverlayApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Already created
        }

        // Find primary monitor
        let monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());

        let mut attrs = Window::default_attributes()
            .with_title("AriaShot Overlay")
            .with_decorations(false)
            .with_resizable(false);

        if let Some(ref mon) = monitor {
            let size = mon.size();
            attrs = attrs.with_inner_size(size);
        }

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                error!("Failed to create window: {}", e);
                self.exit_code = 2;
                event_loop.exit();
                return;
            }
        };

        // macOS: simple fullscreen (covers menu bar/dock without creating a new Space)
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowExtMacOS;
            window.set_simple_fullscreen(true);
        }

        // Linux: borderless fullscreen on the detected monitor
        #[cfg(target_os = "linux")]
        {
            use winit::window::Fullscreen;
            window.set_fullscreen(Some(Fullscreen::Borderless(monitor)));
        }

        // Always on top
        window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);

        // Create softbuffer surface
        let context = softbuffer::Context::new(window.clone()).expect("softbuffer Context");
        let mut sb_surface =
            softbuffer::Surface::new(&context, window.clone()).expect("softbuffer Surface");

        let w = NonZeroU32::new(self.surface.width).unwrap();
        let h = NonZeroU32::new(self.surface.height).unwrap();
        sb_surface.resize(w, h).expect("softbuffer resize");

        self.sb_surface = Some(sb_surface);
        self.window = Some(window.clone());

        // Use Wait mode — only redraw when state changes
        event_loop.set_control_flow(ControlFlow::Wait);

        // Trigger initial render
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.exit_code = 0;
                event_loop.exit();
            }

            WindowEvent::ModifiersChanged(new_modifiers) => {
                self.modifiers = new_modifiers.state();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Escape) => {
                    self.exit_code = 0;
                    event_loop.exit();
                }
                Key::Named(NamedKey::Enter) => {
                    if matches!(self.surface.controller.state, OverlayState::Selected { .. }) {
                        self.copy_and_exit(event_loop);
                    }
                }
                Key::Character(ref c) if c.as_str() == "c" => {
                    let is_cmd_or_ctrl = if cfg!(target_os = "macos") {
                        self.modifiers.super_key()
                    } else {
                        self.modifiers.control_key()
                    };
                    if is_cmd_or_ctrl
                        && matches!(self.surface.controller.state, OverlayState::Selected { .. })
                    {
                        self.copy_and_exit(event_loop);
                    }
                }
                _ => {}
            },

            WindowEvent::CursorMoved { position, .. } => {
                let pt = Point::new(
                    position.x.clamp(0.0, self.surface.width as f64 - 1.0),
                    position.y.clamp(0.0, self.surface.height as f64 - 1.0),
                );
                self.last_cursor = pt;
                self.surface.on_mouse_move(pt);
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => {
                            // Check toolbar hit-test first when in Selected state
                            if let OverlayState::Selected { rect } = self.surface.controller.state {
                                if let Some(action) = FloatingToolbar::hit_test(
                                    rect,
                                    self.surface.width,
                                    self.surface.height,
                                    self.last_cursor,
                                ) {
                                    match action {
                                        ToolbarAction::CopyToClipboard => {
                                            self.copy_and_exit(event_loop);
                                            return;
                                        }
                                        ToolbarAction::Close => {
                                            self.exit_code = 0;
                                            event_loop.exit();
                                            return;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            self.surface.on_mouse_down(self.last_cursor);
                        }
                        ElementState::Released => {
                            self.surface.on_mouse_up();
                        }
                    }
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                self.present_frame();
            }

            _ => {}
        }
    }
}
