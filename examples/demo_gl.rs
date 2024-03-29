use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;
use std::os::raw::c_char;

use thyme::bench;

mod demo;

const OPENGL_MAJOR_VERSION: u8 = 3;
const OPENGL_MINOR_VERSION: u8 = 2;

/// A basic RPG character sheet, using the "plain" OpenGL backend.
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
        .build_windowed(window_builder, &event_loop)?;

    let windowed_context = unsafe {
        windowed_context
            .make_current().map_err(|(_context, e)| e)?
    };

    {
        let gl_context = windowed_context.context();
        gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _)
    }

    // create thyme backend
    let mut renderer = thyme::GLRenderer::new();
    let mut context_builder = thyme::ContextBuilder::with_defaults();

    demo::register_assets(&mut context_builder);

    let window_size = [1280.0, 720.0];
    let mut io = thyme::WinitIo::new(&event_loop, window_size.into())?;
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

            renderer.clear_color(0.5, 0.5, 0.5, 1.0);

            bench::run("thyme", || {
                windowed_context.window().set_cursor_visible(!party.theme_has_mouse_cursor());

                let mut ui = context.create_frame();

                bench::run("frame", || {
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

// this is passed as a fn pointer to gl::DebugMessageCallback
// and cannot be marked as an "unsafe extern"
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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