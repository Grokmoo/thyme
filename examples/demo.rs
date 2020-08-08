use glium::glutin::{self, event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};
use glium::{Display, Surface};

use thyme::{Color, Frame, Align, Point};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/gui.png");
    let image = image::load_from_memory(image_src).unwrap().to_rgba();
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
    let mut context_builder = thyme::ContextBuilder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_texture("gui", &image.into_raw(), image_dims)?;
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

            let mut frame = context.create_frame();

            build_ui(&mut frame);

            renderer.draw_frame(&mut target, frame).unwrap();

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
fn build_ui(ui: &mut Frame) {
    ui.window("window", "main_window", Point::new(300.0, 300.0), |ui| {
        ui.gap(20.0);

        ui.start("textbox")
        .text(SAMPLE_TEXT)
        .text_color(Color::cyan())
        .finish();
        let label = if ui.is_open("window2") {
            "Close Window"
        } else {
            "Open Window"
        };
        if ui.button("button", label).clicked {
            ui.toggle_open("window2");
        }

        let frac = (ui.offset_time_millis("pbar") as f32 / 1_000.0).min(1.0);

        ui.progress_bar("progress_bar", frac);

        if ui.button("button", "Start!").clicked {
            ui.set_base_time_now("pbar");
        }
    });

    ui.start("window")
    .id("window2")
    .size(200.0, 200.0)
    .align(Align::Bot)
    .children(|ui| {
        let result = ui.start("titlebar")
        .children(|ui| {
            ui.label("title", "Window Title");

            if ui.button("close", "").clicked {
                ui.set_open("window2", false);
            }
        }).finish();

        if result.pressed {
            ui.modify("window2", |state| {
                state.moved = state.moved + result.dragged;
            });
        }

        let result = ui.button("handle", "");
        if result.pressed {
            ui.modify("window2", |state| {
                state.resize = state.resize + result.dragged;
            });
        }

        ui.button("flashing_button", "Flash");
    }).finish();
}

const SAMPLE_TEXT: &str = r#"
This is some longer multiline text.
It has explicit line breaks as well as automatic text wrapping.  It also has a different color and a different size than the other text in the window.
Thisisanextremelylonglinewithnospacingtotestoutwhathappenswhenyoudon'thaveanyspacesinyourword.
"#;