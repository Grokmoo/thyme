use std::error::Error;

use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey, ModifiersKeyState};
use winit::window::Window;

use crate::point::Point;
use crate::context::{InputModifiers, Context};
use crate::render::IO;
use crate::KeyEvent;

/**
A Thyme Input/Output adapter for [`winit`](https://github.com/rust-windowing/winit).

This adapter handles events from `winit` and sends them to the Thyme [`Context`](struct.Context.html).
WindowEvents should be passed to this handler, assuming [`Context.wants_mouse`](struct.Context.html#method.wants_mouse)
returns true for the given frame.

# Example
```
fn main_loop(event_loop: winit::EventLoop<()>, thyme: thyme::Context) {
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            // Renderer specific code here

            let mut ui = context.create_frame();
            // create UI here

            // draw the frame and finish up rendering here
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}
```
*/
pub struct WinitIo {
    scale_factor: f32,
    display_size: Point,
}

impl IO for WinitIo {
    fn scale_factor(&self) -> f32 { self.scale_factor }

    fn display_size(&self) -> Point { self.display_size }
}

impl WinitIo {
    /// Creates a new adapter from the given `EventLoop`, with the specified initial display size,
    /// in logical pixels.  This may change over time.
    pub fn new(
        window: &Window,
        logical_display_size: Point,
    ) -> Result<WinitIo, WinitError> {
        let monitor = window.primary_monitor().ok_or(WinitError::PrimaryMonitorNotFound)?;
        let scale_factor = monitor.scale_factor() as f32;
        Ok(WinitIo {
            scale_factor,
            display_size: logical_display_size * scale_factor,
        })
    }

    /// Handles a winit `Event` and passes it to the Thyme [`Context`](struct.Context.html).
    pub fn handle_event(&mut self, context: &mut Context, event: &WindowEvent) {
        use WindowEvent::*;
        match event {
            Resized(size) => {
                let (x, y): (u32, u32) = (*size).into();
                let size: Point = (x as f32, y as f32).into();
                self.display_size = size;
                context.set_display_size(size);
            },
            ModifiersChanged(m) => {
                use ModifiersKeyState::*;
                let shift = m.lshift_state() == Pressed || m.rshift_state() == Pressed;
                let ctrl = m.lcontrol_state() == Pressed || m.rcontrol_state() == Pressed;
                let alt = m.lalt_state() == Pressed || m.ralt_state() == Pressed;
                context.set_input_modifiers(InputModifiers { shift, ctrl, alt });
            },
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scale = *scale_factor as f32;
                self.scale_factor = scale;
                context.set_scale_factor(scale);
            },
            MouseInput { state, button, .. } => {
                let pressed = match state {
                    ElementState::Pressed => true,
                    ElementState::Released => false,
                };

                let index: usize = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    MouseButton::Back => 3,
                    MouseButton::Forward => 4,
                    MouseButton::Other(index) => *index as usize + 5,
                };

                context.set_mouse_pressed(pressed, index);
            },
            MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        // TODO configure line delta
                        context.add_mouse_wheel(Point::new(*x, *y), true);
                    }, MouseScrollDelta::PixelDelta(pos) => {
                        let x = pos.x as f32;
                        let y = pos.y as f32;
                        context.add_mouse_wheel(Point::new(x, y), false);
                    }
                }
            },
            CursorMoved { position, .. } => {
                context.set_mouse_pos((position.x as f32 / self.scale_factor, position.y as f32 / self.scale_factor).into());
            },
            KeyboardInput { event, .. } => {
                if let Some(str) = event.text.as_ref() {
                    if let ElementState::Pressed = event.state {
                        for c in str.chars() {
                            context.push_character(c);
                        }
                    }
                }

                match &event.logical_key {
                    Key::Named(named_key) => {
                        if let ElementState::Released = event.state {
                            if let Some(key) = key_event(*named_key) {
                                context.push_key_event(key);
                            }
                        }
                    },
                    Key::Character(_) | Key::Unidentified(_) | Key::Dead(_) => (),
                }
            },
            _ => (),
        }
    }
}

/// An error of several types originating from winit Windowing functions
#[derive(Debug)]
pub enum WinitError {
    /// No primary monitor is found
    PrimaryMonitorNotFound,

    /// Internal OS error forwarded to winit
    Os(winit::error::OsError),

    /// An error in the creation or execution of the EventLoop
    EventLoop(winit::error::EventLoopError),

    /// An error getting the window handle associated with a window
    HandleError(winit::raw_window_handle::HandleError),
}

impl std::fmt::Display for WinitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::WinitError::*;
        match self {
            PrimaryMonitorNotFound => write!(f, "Primary monitor not found."),
            Os(e) => write!(f, "OS Error: {}", e),
            EventLoop(e) => write!(f, "Event Loop error: {}", e),
            HandleError(e) => write!(f, "Window handle error: {}", e),
        }
    }
}

impl Error for WinitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::WinitError::*;
        match self {
            PrimaryMonitorNotFound => None,
            Os(e) => Some(e),
            EventLoop(e) => Some(e),
            HandleError(e) => Some(e),
        }
    }
}

fn key_event(input: NamedKey) -> Option<KeyEvent> {
    use NamedKey::*;
    Some(match input {
        Insert => KeyEvent::Insert,
        Home => KeyEvent::Home,
        Delete => KeyEvent::Delete,
        End => KeyEvent::End,
        PageDown => KeyEvent::PageDown,
        PageUp => KeyEvent::PageUp,
        ArrowLeft => KeyEvent::Left,
        ArrowUp => KeyEvent::Up,
        ArrowRight => KeyEvent::Right,
        ArrowDown => KeyEvent::Down,
        Backspace => KeyEvent::Back,
        Enter => KeyEvent::Return,
        Space => KeyEvent::Space,
        Escape => KeyEvent::Escape,
        Tab => KeyEvent::Tab,
        F1 => KeyEvent::F1,
        F2 => KeyEvent::F2,
        F3 => KeyEvent::F3,
        F4 => KeyEvent::F4,
        F5 => KeyEvent::F5,
        F6 => KeyEvent::F6,
        F7 => KeyEvent::F7,
        F8 => KeyEvent::F8,
        F9 => KeyEvent::F9,
        F10 => KeyEvent::F10,
        F11 => KeyEvent::F11,
        F12 => KeyEvent::F12,
        _ => return None,
    })
}