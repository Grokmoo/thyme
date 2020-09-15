use std::path::Path;

use winit::{event::{Event, WindowEvent}, event_loop::{EventLoop, ControlFlow}};
use thyme::{Align, bench};

mod demo;

/// A basic RPG character sheet, using the wgpu backend.
/// This file contains the application setup code and wgpu specifics.
/// the `demo.rs` file contains the Thyme UI code and logic.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    let window_size = [1280.0, 720.0];
    let events_loop = EventLoop::new();

    // create winit window
    let window = winit::window::WindowBuilder::new()
        .with_title("Thyme WGPU Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(window_size[0], window_size[1]))
        .build(&events_loop)
        .unwrap();

    // hide the default cursor
    window.set_cursor_visible(false);

    // setup WGPU
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let (_adapter, device, queue) = futures::executor::block_on(setup_wgpu(&instance, &surface));
    let sc_desc = swapchain_desc(window_size[0] as u32, window_size[1] as u32);
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    // create thyme backend
    let mut io = thyme::WinitIo::new(&events_loop, window_size.into());
    let mut renderer = thyme::WgpuRenderer::new(std::rc::Rc::clone(&device), std::rc::Rc::clone(&queue));
    let mut context_builder = thyme::ContextBuilder::new(&mut renderer, &mut io);

    // register resources in thyme by reading from files.  this enables live reload.
    context_builder.register_theme_from_files(
        &[
            Path::new("examples/data/theme-base.yml"),
            Path::new("examples/data/theme-fantasy.yml"),
            Path::new("examples/data/theme-demo.yml"),
        ],
        serde_yaml::from_str::<serde_yaml::Value>
    )?;
    context_builder.register_texture_from_file("pixel", Path::new("examples/data/images/gui-pixel.png"));
    context_builder.register_texture_from_file("fantasy", Path::new("examples/data/images/gui-fantasy.png"));
    context_builder.register_font_from_file("roboto", Path::new("examples/data/fonts/Roboto-Medium.ttf"));
    let mut context = context_builder.build()?;

    let mut party = demo::Party::default();

    // run main loop
    events_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                let frame_start = std::time::Instant::now();

                if party.take_reload_assets() {
                    context.rebuild(&mut renderer).unwrap();
                }

                let frame = swap_chain.get_current_frame().unwrap().output;
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                bench::run("thyme", || {
                    let mut ui = context.create_frame();
    
                    bench::run("frame", || {
                        // show a custom cursor.  it automatically inherits mouse presses in its state
                        ui.set_mouse_cursor("gui/cursor", Align::TopLeft);
                        demo::build_ui(&mut ui, &mut party);
                    });

                    bench::run("draw", || {
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
                    });
                });

                *control_flow = ControlFlow::WaitUntil(frame_start + std::time::Duration::from_millis(16));
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
) -> (wgpu::Adapter, std::rc::Rc<wgpu::Device>, std::rc::Rc<wgpu::Queue>) {
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

    (adapter, std::rc::Rc::new(device), std::rc::Rc::new(queue))
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