use glutin;
use glutin::event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use glutin::event_loop::ControlFlow;
use std::os::raw::c_char;
use thyme::{bench, Align, Point};
use thyme::{Context, InputModifiers};

mod demo;

struct IOData {
    scale_factor: f32,
    display_size: Point,
}

impl thyme::IO for IOData {
    fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    fn display_size(&self) -> Point {
        self.display_size
    }
}

impl IOData {
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
            }
            ModifiersChanged(m) => {
                context.set_input_modifiers(InputModifiers {
                    shift: m.shift(),
                    ctrl: m.ctrl(),
                    alt: m.alt(),
                });
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let scale = *scale_factor as f32;
                self.scale_factor = scale;
                context.set_scale_factor(scale);
            }
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
            }
            MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        // TODO configure line delta
                        context.add_mouse_wheel(Point::new(*x * 10.0, *y * 10.0));
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        let x = pos.x as f32;
                        let y = pos.y as f32;
                        context.add_mouse_wheel(Point::new(x, y));
                    }
                }
            }
            CursorMoved { position, .. } => {
                context.set_mouse_pos(
                    (
                        position.x as f32 / self.scale_factor,
                        position.y as f32 / self.scale_factor,
                    )
                        .into(),
                );
            }
            ReceivedCharacter(c) => {
                context.push_character(*c);
            }
            _ => (),
        }
    }
}

const OPENGL_MAJOR_VERSION: u8 = 3;
const OPENGL_MINOR_VERSION: u8 = 2;

#[no_mangle]
pub extern "system" fn debug_callback(
    _: gl::types::GLenum,
    err_type: gl::types::GLenum,
    id: gl::types::GLuint,
    severity: gl::types::GLenum,
    _: gl::types::GLsizei,
    message: *const c_char,
    _: *mut std::ffi::c_void,
) {
    match err_type {
        gl::DEBUG_TYPE_ERROR | gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => unsafe {
            let err_text = std::ffi::CStr::from_ptr(message);
            println!(
                "Type: {:?} ID: {:?} Severity: {:?}:\n  {:#?}",
                err_type,
                id,
                severity,
                err_text.to_str().unwrap()
            );
        },
        _ => {}
    }
}

/// A basic RPG character sheet, using the wgpu backend.
/// This file contains the application setup code and wgpu specifics.
/// the `demo.rs` file contains the Thyme UI code and logic.
/// A simple party creator and character sheet for an RPG.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize our very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    // create glium display
    let event_loop = glutin::event_loop::EventLoop::new();
    let window_builder = glutin::window::WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(glutin::dpi::LogicalSize::new(1280.0, 720.0));

    let windowed_context = glutin::ContextBuilder::new()
        .with_gl(glutin::GlRequest::Specific(
            glutin::Api::OpenGl,
            (OPENGL_MAJOR_VERSION, OPENGL_MINOR_VERSION),
        ))
        .build_windowed(window_builder, &event_loop)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    // hide the default cursor
    windowed_context.window().set_cursor_visible(false);
    let _gl = {
        let gl_context = windowed_context.context();
        gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _)
    };

    if OPENGL_MAJOR_VERSION >= 4 && OPENGL_MINOR_VERSION >= 3 {
        // Debug message callback is only available since 4.3
        // Not critical since shader info (what we are most interested in)
        // is supported in lower versions anyway.
        unsafe { gl::DebugMessageCallback(Some(debug_callback), std::ptr::null()) };
    }

    // create thyme backend
    let mut renderer = thyme::GLRenderer::new();
    let mut context_builder = thyme::ContextBuilder::with_defaults();

    demo::register_assets(&mut context_builder);

    let mut io = IOData {
        scale_factor: 1.0,
        display_size: Point::new(1280.0, 780.0),
    };
    let mut context = context_builder.build(&mut renderer, &mut io)?;
    let mut party = demo::Party::default();

    let mut last_frame = std::time::Instant::now();
    let frame_time = std::time::Duration::from_millis(16);

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            if std::time::Instant::now() > last_frame + frame_time {
                windowed_context.window().request_redraw();
            }
            *control_flow = ControlFlow::WaitUntil(last_frame + frame_time);
        }
        Event::RedrawRequested(_) => {
            last_frame = std::time::Instant::now();

            party.check_context_changes(&mut context, &mut renderer);

            renderer.clear_color(0.21404, 0.21404, 0.21404, 1.0); // manual sRGB conversion for 0.5

            bench::run("thyme", || {
                let mut ui = context.create_frame();

                bench::run("frame", || {
                    // show a custom cursor.  it automatically inherits mouse presses in its state
                    ui.set_mouse_cursor("gui/cursor", Align::TopLeft);
                    demo::build_ui(&mut ui, &mut party);
                });

                bench::run("draw", || {
                    renderer.draw_frame(ui);
                });
            });

            windowed_context.swap_buffers().unwrap();
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}
