use glium::glutin::{
    self,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop}, window::WindowBuilder
};
use glium::{Display, Surface};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/pixel.png");
    let image = image::load_from_memory(image_src).unwrap().to_rgba();
    let theme_src = include_str!("data/themes/base.yml");
    let theme: serde_yaml::Value = serde_yaml::from_str(theme_src)?;
    let window_size = [1280.0, 720.0];

    // create glium display
    let event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new();
    let builder = WindowBuilder::new()
        .with_title("Thyme Glium Demo")
        .with_inner_size(glutin::dpi::LogicalSize::new(window_size[0], window_size[1]));
    let display = Display::new(builder, context, &event_loop)?;

    // create thyme backend
    let mut io = thyme::WinitIo::new(&event_loop, window_size.into())?;
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut context_builder = thyme::ContextBuilder::new(thyme::BuildOptions { enable_live_reload: false });

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_theme(theme)?;
    context_builder.register_texture("pixel", image.into_raw(), image_dims);
    context_builder.register_font("Roboto-Medium", font_src.to_vec());
    let mut context = context_builder.build(&mut renderer, &mut io)?;

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

            let mut ui = context.create_frame();

            ui.window("window", |ui| {
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