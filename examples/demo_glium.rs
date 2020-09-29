use winit::{event::{Event, WindowEvent}, event_loop::{EventLoop, ControlFlow}};
use thyme::{Align, bench};

mod demo;

/// A basic RPG character sheet, using the wgpu backend.
/// This file contains the application setup code and wgpu specifics.
/// the `demo.rs` file contains the Thyme UI code and logic.
/// A simple party creator and character sheet for an RPG.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use glium::glutin::{self, window::WindowBuilder};
    use glium::{Display, Surface};

    // initialize our very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    let window_size = [1280.0, 720.0];
    let events_loop = EventLoop::new();

    // create glium display
    let context = glutin::ContextBuilder::new();
    let builder = WindowBuilder::new()
        .with_title("Thyme Demo")
        .with_inner_size(glutin::dpi::LogicalSize::new(window_size[0], window_size[1]));
    let display = Display::new(builder, context, &events_loop)?;

    // hide the default cursor
    display.gl_window().window().set_cursor_visible(false);

    // create thyme backend
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut io = thyme::WinitIo::new(&events_loop, window_size.into());
    let mut context_builder = thyme::ContextBuilder::with_defaults();

    demo::register_assets(&mut context_builder);
    
    let mut context = context_builder.build(&mut renderer, &mut io)?;

    let mut party = demo::Party::default();

    // run main loop
    events_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let frame_start = std::time::Instant::now();

            party.check_context_changes(&mut context, &mut renderer);

            let mut target = display.draw();
            target.clear_color(0.21404, 0.21404, 0.21404, 1.0); // manual sRGB conversion for 0.5

            bench::run("thyme", || {
                let mut ui = context.create_frame();

                bench::run("frame", || {
                    // show a custom cursor.  it automatically inherits mouse presses in its state
                    ui.set_mouse_cursor("gui/cursor", Align::TopLeft);
                    demo::build_ui(&mut ui, &mut party);
                });

                bench::run("draw", || {
                    renderer.draw_frame(&mut target, ui).unwrap();
                });
            });

            target.finish().unwrap();

            *control_flow = ControlFlow::WaitUntil(frame_start + std::time::Duration::from_millis(16));
        },
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}