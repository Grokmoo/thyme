use winit::{event::{Event, WindowEvent}, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder, dpi::LogicalSize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize very basic logger so error messages go to stdout
    thyme::log::init_all().unwrap();

    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/gui-minimal.png");
    let image = image::load_from_memory(image_src).unwrap().to_rgba();
    let theme_src = include_str!("data/theme-minimal.yml");
    let theme: serde_yaml::Value = serde_yaml::from_str(theme_src)?;
    let window_size = [1280.0, 720.0];

    // create winit window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Thyme WGPU Demo")
        .with_inner_size(LogicalSize::new(window_size[0], window_size[1]))
        .build(&event_loop);

    // TODO setup WGPU

    // create thyme backend
    let mut io = thyme::WinitIo::new(&event_loop, window_size.into());
    let mut renderer = thyme::WgpuRenderer::new();
    let mut context_builder = thyme::ContextBuilder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_texture("gui", &image.into_raw(), image_dims)?;
    context_builder.register_font_source("roboto", font_src.to_vec())?;
    let mut context = context_builder.build()?;

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            // TODO renderer setup

            let mut ui = context.create_frame();

            ui.window("window", |ui| {
                ui.gap(20.0);
        
                ui.button("label", "Hello, World!");
            });

            // TODO renderer draw
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}