use winit::{application::ApplicationHandler, dpi::LogicalSize, event::WindowEvent, window::Window};
use thyme::{bench, Context, ContextBuilder, GliumRenderer, WinitError, WinitIo};

mod demo;

/// A basic RPG character sheet, using the glium backend.
/// This file contains the application setup code and wgpu specifics.
/// the `demo.rs` file contains the Thyme UI code and logic.
/// A simple party creator and character sheet for an RPG.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize our very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    let window_size = [1280.0, 720.0];

    let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build().map_err(|e| thyme::Error::Winit(WinitError::EventLoop(e)))?;

    let attrs = Window::default_attributes()
        .with_title("Thyme Demo")
        .with_inner_size(LogicalSize::new(window_size[0], window_size[1]));

    // create glium display
    let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
        .set_window_builder(attrs)
        .build(&event_loop);

    // create thyme backend
    let mut renderer = GliumRenderer::new(&display)?;
    let mut io = WinitIo::new(&window, window_size.into())?;
    let mut context_builder = ContextBuilder::with_defaults();

    demo::register_assets(&mut context_builder);

    let context = context_builder.build(&mut renderer, &mut io)?;

    let party = demo::Party::default();

    let mut app = AppRunner { io, renderer, context, display, window, party, frames: 0 };

    let start = std::time::Instant::now();

    event_loop.run_app(&mut app)?;

    let finish = std::time::Instant::now();

    log::warn!("Drew {} frames in {:.2}s", app.frames, (finish - start).as_secs_f32());

    Ok(())
}

struct AppRunner {
    io: WinitIo,
    renderer: GliumRenderer,
    context: Context,
    display: glium::Display<glium::glutin::surface::WindowSurface>,
    window: winit::window::Window,
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
        use glium::Surface;
        match event {
            WindowEvent::RedrawRequested => {
                self.party.check_context_changes(&mut self.context, &mut self.renderer);

                let mut target = self.display.draw();
                target.clear_color(0.21, 0.21, 0.21, 1.0);
    
                bench::run("thyme", || {
                    self.window.set_cursor_visible(!self.party.theme_has_mouse_cursor());
    
                    let mut ui = self.context.create_frame();
    
                    bench::run("frame", || {
                        demo::build_ui(&mut ui, &mut self.party);
                    });
    
                    bench::run("draw", || {
                        self.renderer.draw_frame(&mut target, ui).unwrap();
                    });
                });
    
                target.finish().unwrap();
                self.frames += 1;
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => {
                self.io.handle_event(&mut self.context, &event);
            }
        }
    }
}