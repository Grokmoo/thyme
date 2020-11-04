use std::sync::Arc;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    dpi::LogicalSize
};

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

    // create winit window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Thyme WGPU Demo")
        .with_inner_size(LogicalSize::new(window_size[0], window_size[1]))
        .build(&event_loop)
        .unwrap();

    // setup WGPU
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let (_adapter, device, queue) = futures::executor::block_on(setup_wgpu(&instance, &surface));
    let sc_desc = swapchain_desc(window_size[0] as u32, window_size[1] as u32);
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    // create thyme backend
    let mut io = thyme::WinitIo::new(&event_loop, window_size.into())?;
    let mut renderer = thyme::WgpuRenderer::new(Arc::clone(&device), Arc::clone(&queue));
    let mut context_builder = thyme::ContextBuilder::new(thyme::BuildOptions { enable_live_reload: false });

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_theme(theme)?;
    context_builder.register_texture("pixel", image.into_raw(), image_dims);
    context_builder.register_font("Roboto-Medium", font_src.to_vec());
    let mut context = context_builder.build(&mut renderer, &mut io)?;

    // run main loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                let frame = swap_chain.get_current_frame().unwrap().output;
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let mut ui = context.create_frame();

                ui.window("window", |ui| {
                    ui.gap(20.0);
            
                    ui.button("label", "Hello, World!");
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    renderer.draw_frame(ui, &mut render_pass);
                }

                queue.submit(Some(encoder.finish()));
            },
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            event => {
                // recreate swap chain on resize, but also still pass the event to thyme
                if let Event::WindowEvent { event: WindowEvent::Resized(_), ..} = event {
                    let size: (u32, u32) = window.inner_size().into();

                    let sc_desc = swapchain_desc(size.0, size.1);
                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
                }

                io.handle_event(&mut context, &event);
            }
        }
    })
}

async fn setup_wgpu(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface
) -> (wgpu::Adapter, Arc<wgpu::Device>, Arc<wgpu::Queue>) {
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::Default,
        // Request an adapter which can render to our surface
        compatible_surface: Some(&surface),
    }).await.unwrap();
    
    // Create the logical device and command queue
    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
            shader_validation: true,
        },
        None,
    ).await.expect("Failed to create WGPU device");

    (adapter, Arc::new(device), Arc::new(queue))
}

fn swapchain_desc(width: u32, height: u32) -> wgpu::SwapChainDescriptor {
    wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width,
        height,
        present_mode: wgpu::PresentMode::Mailbox,
    }
}