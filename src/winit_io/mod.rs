use winit::event::{Event, WindowEvent, MouseButton, MouseScrollDelta, ElementState};
use winit::event_loop::EventLoop;

use crate::point::Point;
use crate::context::Context;
use crate::render::IO;

/// A Thyme Input/Output adapter for [`winit`](https://github.com/rust-windowing/winit).
///
/// This adapter handles events from `winit` and sends them to the Thyme [`Context`](struct.Context.html).
/// WindowEvents should be passed to this handler, assuming [`Context.wants_mouse`](struct.Context.html#method.wants_mouse)
/// returns true for the given frame.
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
    pub fn new<T>(event_loop: &EventLoop<T>, logical_display_size: Point) -> WinitIo {
        let monitor = event_loop.primary_monitor();
        let scale_factor = monitor.scale_factor() as f32;
        WinitIo {
            scale_factor,
            display_size: logical_display_size * scale_factor,
        }
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
                        context.add_mouse_wheel(Point::new(*x, *y));
                    }, MouseScrollDelta::PixelDelta(pos) => {
                        let x = pos.x as f32;
                        let y = pos.y as f32;
                        context.add_mouse_wheel(Point::new(x, y));
                    }
                }
            },
            CursorMoved { position, .. } => {
                context.set_mouse_pos((position.x as f32 / self.scale_factor, position.y as f32 / self.scale_factor).into());
            },
            ReceivedCharacter(c) => {
                context.push_character(*c);
            }
            _ => (),
        }
    }
}