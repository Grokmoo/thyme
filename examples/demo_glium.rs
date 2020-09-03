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
    thyme::log::init_all().unwrap();

    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/gui-minimal.png");
    let image = image::load_from_memory(image_src).unwrap().to_rgba();

    // a very simple method of splitting up our theme into two files for readability
    let theme_base_src = include_str!("data/theme-minimal.yml");
    let theme_demo_src = include_str!("data/theme.yml");
    let theme_src = format!("{}\n{}", theme_base_src, theme_demo_src);

    let theme: serde_yaml::Value = serde_yaml::from_str(&theme_src)?;
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

    // create thyme renderer
    let mut renderer = thyme::GliumRenderer::new(&display)?;

    // create thyme backend
    let mut io = thyme::WinitIo::new(&events_loop, window_size.into());
    let mut context_builder = thyme::ContextBuilder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_texture("gui", &image.into_raw(), image_dims)?;
    context_builder.register_font("roboto", font_src.to_vec())?;
    let mut context = context_builder.build()?;

    let mut party = demo::Party::default();

    // run main loop
    events_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let frame_start = std::time::Instant::now();

            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

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