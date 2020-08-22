use winit::event::{Event, WindowEvent, MouseButton, ElementState};
use winit::event_loop::EventLoop;

use crate::point::Point;
use crate::context::Context;
use crate::render::IO;

pub struct WinitIo {
    scale_factor: f32,
    display_size: Point,
}

impl IO for WinitIo {
    fn scale_factor(&self) -> f32 { self.scale_factor }

    fn display_size(&self) -> Point { self.display_size }
}

impl WinitIo {
    pub fn new<T>(event_loop: &EventLoop<T>, logical_display_size: Point) -> WinitIo {
        let monitor = event_loop.primary_monitor();
        let scale_factor = monitor.scale_factor() as f32;
        WinitIo {
            scale_factor,
            display_size: logical_display_size * scale_factor,
        }
    }

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
            CursorMoved { position, .. } => {
                context.set_mouse_pos((position.x as f32, position.y as f32).into());
            },
            ReceivedCharacter(c) => {
                context.push_character(*c);
            }
            _ => (),
        }
    }
}