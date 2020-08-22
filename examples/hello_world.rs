use glium::glutin::{
    self,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop}, window::WindowBuilder
};
use glium::{Display, Surface};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/gui.png");
    let image = image::load_from_memory(image_src).unwrap().to_rgba();
    let theme_src = include_str!("data/theme-minimal.yml");
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
    let mut io = thyme::WinitIo::new(&event_loop, window_size.into());
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut context_builder = thyme::ContextBuilder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_texture("gui", &image.into_raw(), image_dims)?;
    context_builder.register_font_source("roboto", font_src.to_vec())?;
    let mut context = context_builder.build()?;

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

            let mut ui = context.create_frame();

            ui.window("window", "window", |ui| {
                ui.gap(20.0);
        
                ui.button("label", "Hello, World!");
            });

            renderer.draw_frame(&mut target, ui).unwrap();

            target.finish().unwrap();
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}