use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version};
use glutin::surface::{Surface, WindowSurface, GlSurface};
use glutin_winit::DisplayBuilder;
use glutin::display::{GlDisplay, GetGlDisplay};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::window::Window;
use winit::raw_window_handle::HasWindowHandle;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::os::raw::c_char;

use thyme::{bench, Context, GLRenderer, WinitIo};

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
    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build()?;

    let attrs = Window::default_attributes()
        .with_title("Thyme GL Demo")
        .with_inner_size(LogicalSize::new(1280, 720));

    let display_builder = DisplayBuilder::new().with_window_attributes(Some(attrs));
    let config_template_builder = ConfigTemplateBuilder::new();

    let (window, gl_config) = display_builder.build(&event_loop, config_template_builder, |mut configs| {
        configs.next().unwrap()
    })?;
    let window = window.unwrap();

    let window_handle = window.window_handle()?;
    let raw_window_handle = window_handle.as_raw();

    let (width, height): (u32, u32) = window.inner_size().into();
    let attrs =
        glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
            .build(
                raw_window_handle,
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            );

    let surface = unsafe {
        gl_config.display().create_window_surface(&gl_config, &attrs).unwrap()
    };

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(OPENGL_MAJOR_VERSION, OPENGL_MINOR_VERSION))))
        .build(Some(raw_window_handle));

    let windowed_context = unsafe {
        gl_config.display().create_context(&gl_config, &context_attributes)?
    };

    let display_context = windowed_context.make_current(&surface)?;

    {
        let gl_context = display_context.display();
        gl::load_with(|ptr| {
            let c_str = CString::new(ptr).unwrap();
            gl_context.get_proc_address(&c_str) as *const _
        })
    }

    // create thyme backend
    let mut renderer = thyme::GLRenderer::new();
    let mut context_builder = thyme::ContextBuilder::with_defaults();

    demo::register_assets(&mut context_builder);

    let window_size = [1280.0, 720.0];
    let mut io = thyme::WinitIo::new(&window, window_size.into())?;
    let context = context_builder.build(&mut renderer, &mut io)?;
    let party = demo::Party::default();

    let mut app = AppRunner { io, renderer, context, window, surface, display_context, party, frames: 0 };

    let start = std::time::Instant::now();
    event_loop.run_app(&mut app)?;
    let finish = std::time::Instant::now();

    log::warn!("Drew {} frames in {:.2}s", app.frames, (finish - start).as_secs_f32());

    Ok(())
}

struct AppRunner {
    io: WinitIo,
    renderer: GLRenderer,
    context: Context,
    window: winit::window::Window,
    surface: Surface<WindowSurface>,
    display_context: PossiblyCurrentContext,
    party: demo::Party,
    frames: u32,
}

impl ApplicationHandler for AppRunner {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) { }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                self.party.check_context_changes(&mut self.context, &mut self.renderer);

                self.renderer.clear_color(0.5, 0.5, 0.5, 1.0);

                bench::run("thyme", || {
                    self.window.set_cursor_visible(!self.party.theme_has_mouse_cursor());

                    let mut ui = self.context.create_frame();

                    bench::run("frame", || {
                        demo::build_ui(&mut ui, &mut self.party);
                    });

                    bench::run("draw", || {
                        self.renderer.draw_frame(ui);
                    });
                });

                self.surface.swap_buffers(&self.display_context).unwrap();
                self.frames += 1;
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => {
                self.io.handle_event(&mut self.context, &event);
            }
        }
    }
}

// this is passed as a fn pointer to gl::DebugMessageCallback
// and cannot be marked as an "unsafe extern"
#[unsafe(no_mangle)]
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