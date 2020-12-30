use winit::{event::{Event, WindowEvent}, event_loop::{EventLoop, ControlFlow}};
use thyme::{bench};

mod demo;

/// A basic RPG character sheet, using the glium backend.
/// This file contains the application setup code and wgpu specifics.
/// the `demo.rs` file contains the Thyme UI code and logic.
/// A simple party creator and character sheet for an RPG.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use glium::glutin::{window::WindowBuilder};
    use glium::{Display, Surface};

    // initialize our very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    let window_size = [1280.0, 720.0];
    let events_loop = EventLoop::new();

    // create glium display
    let builder = WindowBuilder::new()
        .with_title("Thyme Demo")
        .with_inner_size(glium::glutin::dpi::LogicalSize::new(window_size[0], window_size[1]));
    let context = glium::glutin::ContextBuilder::new();
    let display = Display::new(builder, context, &events_loop)?;

    // create thyme backend
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut io = thyme::WinitIo::new(&events_loop, window_size.into())?;
    let mut context_builder = thyme::ContextBuilder::with_defaults();

    demo::register_assets(&mut context_builder);

    let mut context = context_builder.build(&mut renderer, &mut io)?;

    let mut party = demo::Party::default();

    let mut last_frame = std::time::Instant::now();
    let frame_time = std::time::Duration::from_millis(16);
    
    // run main loop
    events_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            if std::time::Instant::now() > last_frame + frame_time {
                display.gl_window().window().request_redraw();
            }
            *control_flow = ControlFlow::WaitUntil(last_frame + frame_time);
        },
        Event::RedrawRequested(_) => {
            last_frame = std::time::Instant::now();

            party.check_context_changes(&mut context, &mut renderer);

            let mut target = display.draw();
            target.clear_color(0.21404, 0.21404, 0.21404, 1.0); // manual sRGB conversion for 0.5

            bench::run("thyme", || {
                display.gl_window().window().set_cursor_visible(!party.theme_has_mouse_cursor());

                let mut ui = context.create_frame();

                bench::run("frame", || {
                    demo::build_ui(&mut ui, &mut party);
                });

                bench::run("draw", || {
                    renderer.draw_frame(&mut target, ui).unwrap();
                });
            });

            target.finish().unwrap();
        },
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}
