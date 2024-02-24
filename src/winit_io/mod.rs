use std::error::Error;

use winit::event::{ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;

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
    pub fn new<T>(
        event_loop: &EventLoop<T>,
        logical_display_size: Point,
    ) -> Result<WinitIo, WinitError> {
        let monitor = event_loop.primary_monitor().ok_or(WinitError::PrimaryMonitorNotFound)?;
        let scale_factor = monitor.scale_factor() as f32;
        Ok(WinitIo {
            scale_factor,
            display_size: logical_display_size * scale_factor,
        })
    }

    /// Handles a winit `Event` and passes it to the Thyme [`Context`](struct.Context.html).
    pub fn handle_event<T>(&mut self, context: &mut Context, event: &Event<T>) {
        let event = match event {
            Event::WindowEvent { event, .. } => event,
            _ => return,
        };

        use WindowEvent::*;
        match event {
            Resized(size) => {
                let (x, y): (u32, u32) = (*size).into();
                let size: Point = (x as f32, y as f32).into();
                self.display_size = size;
                context.set_display_size(size);
            },
            ModifiersChanged(m) => {
                context.set_input_modifiers(InputModifiers {
                    shift: m.shift(),
                    ctrl: m.ctrl(),
                    alt: m.alt(),
                });
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
                    MouseButton::Other(index) => *index as usize + 3,
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
            ReceivedCharacter(c) => {
                context.push_character(*c);
            },
            KeyboardInput { input, .. } => {
                if let Some(event) = key_event(input.virtual_keycode) {
                    context.push_key_event(event);
                }
            },
            _ => (),
        }
    }
}

#[derive(Debug)]
pub enum WinitError {
    PrimaryMonitorNotFound,
    Os(winit::error::OsError),
}

impl std::fmt::Display for WinitError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::WinitError::*;
        match self {
            PrimaryMonitorNotFound => write!(f, "Primary monitor not found."),
            Os(e) => write!(f, "OS Error: {}", e),
        }
    }
}

impl Error for WinitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::WinitError::*;
        match self {
            PrimaryMonitorNotFound => None,
            Os(e) => Some(e),
        }
    }
}

fn key_event(input: Option<VirtualKeyCode>) -> Option<KeyEvent> {
    let input = match input {
        None => return None,
        Some(i) => i,
    };

    use VirtualKeyCode::*;
    Some(match input {
        Insert => KeyEvent::Insert,
        Home => KeyEvent::Home,
        Delete => KeyEvent::Delete,
        End => KeyEvent::End,
        PageDown => KeyEvent::PageDown,
        PageUp => KeyEvent::PageUp,
        Left => KeyEvent::Left,
        Up => KeyEvent::Up,
        Right => KeyEvent::Right,
        Down => KeyEvent::Down,
        Back => KeyEvent::Back,
        Return => KeyEvent::Return,
        Space => KeyEvent::Space,
        Escape => KeyEvent::Escape,
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