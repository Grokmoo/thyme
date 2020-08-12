use winit::event::{Event, WindowEvent, MouseButton, ElementState};

use crate::context::Context;
use crate::render::IO;

pub struct WinitIo {}

impl IO for WinitIo {}

impl Default for WinitIo {
    fn default() -> Self { WinitIo::new() }
}

impl WinitIo {
    pub fn new() -> WinitIo {
        WinitIo {}
    }

    pub fn handle_event<T>(&self, context: &mut Context, event: &Event<T>) {
        let event = match event {
            Event::WindowEvent { event, .. } => event,
            _ => return,
        };

        use WindowEvent::*;
        match event {
            Resized(size) => {
                let (x, y): (u32, u32) = (*size).into();
                context.set_display_size((x as f32, y as f32).into());
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