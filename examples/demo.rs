use glium::glutin::{self, event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};
use glium::{Display, Surface};

use thyme::{Color, Frame, Widget, Align};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/gui.png");
    let image = image::load_from_memory(image_src).unwrap();
    let theme_src = include_str!("data/theme.yml");
    let theme: serde_yaml::Value = serde_yaml::from_str(theme_src)?;
    let window_size = [1280.0, 720.0];

    // create glium display
    let event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new();
    let builder = WindowBuilder::new()
        .with_title("Thyme Demo")
        .with_inner_size(glutin::dpi::LogicalSize::new(window_size[0], window_size[1]));
    let display = Display::new(builder, context, &event_loop)?;

    // create thyme backend
    let mut io = thyme::WinitIo::new();
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut context_builder = thyme::Builder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    context_builder.register_texture("gui", image.to_rgba())?;
    context_builder.register_font_source("roboto", font_src.to_vec())?;
    let mut context = context_builder.build(window_size.into())?;

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

            let (mut frame, mut root) = context.create_frame();

            build_ui(&mut frame, &mut root);

            let draw_data = frame.render(root);
            renderer.draw(&mut target, &draw_data).unwrap();

            target.finish().unwrap();
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}

/// The function to build the Thyme user interface.  Called once each frame.  This
/// example demonstrates a combination of Rust layout and styling as well as use
/// of the theme definition file, loaded above
fn build_ui(ui: &mut Frame, root: &mut Widget) {
    root.start(ui, "window")
    .size(300.0, 300.0)
    .align(Align::Center)
    .children(|parent| {
        parent.label(ui, "title", "Window Title");
        parent.gap(20.0);

        parent.start(ui, "label")
        .text("This is some smaller text")
        .text_color(Color::cyan())
        .finish(ui);

        if parent.button(ui, "button", "Toggle").clicked {
           ui.toggle_open("window2");
        }
    })
    .finish(ui);

    root.start(ui, "window")
    .id("window2")
    .size(200.0, 200.0)
    .align(Align::Bot)
    .children(|parent| {
        parent.label(ui, "title", "Subwindow");
    })
    .finish(ui);
}