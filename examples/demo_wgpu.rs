use std::sync::Arc;

use winit::{event::{Event, WindowEvent}, event_loop::{EventLoop, ControlFlow}};
use thyme::{bench};

mod demo;

/// A basic RPG character sheet, using the wgpu backend.
/// This file contains the application setup code and wgpu specifics.
/// the `demo.rs` file contains the Thyme UI code and logic.
/// A simple party creator and character sheet for an RPG.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use winit::{ window::WindowBuilder };

    // initialize our very basic logger so error messages go to stdout
    thyme::log::init(log::Level::Warn).unwrap();

    let window_size = [1280.0, 720.0];
    let events_loop = EventLoop::new();

    // create winit window
    let window = WindowBuilder::new()
        .with_title("Thyme WGPU Demo")
        .with_inner_size(winit::dpi::LogicalSize::new(window_size[0], window_size[1]))
        .build(&events_loop)?;

    // setup WGPU
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let (_adapter, device, queue) = futures::executor::block_on(setup_wgpu(&instance, &surface));
    let sc_desc = swapchain_desc(window_size[0] as u32, window_size[1] as u32);
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    // create thyme backend
    let mut renderer = thyme::WgpuRenderer::new(Arc::clone(&device), Arc::clone(&queue));
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
                window.request_redraw();
            }
            *control_flow = ControlFlow::WaitUntil(last_frame + frame_time);
        },
        Event::RedrawRequested(_) => {
            last_frame = std::time::Instant::now();

            party.check_context_changes(&mut context, &mut renderer);

            let frame = swap_chain.get_current_frame().unwrap().output;
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            bench::run("thyme", || {
                window.set_cursor_visible(!party.theme_has_mouse_cursor());

                let mut ui = context.create_frame();

                bench::run("frame", || {
                    demo::build_ui(&mut ui, &mut party);
                });

                bench::run("draw", || {
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[wgpu::RenderPassColorAttachment {
                                view: &frame.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }),
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
    })
}

async fn setup_wgpu(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface
) -> (wgpu::Adapter, Arc<wgpu::Device>, Arc<wgpu::Queue>) {
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        // Request an adapter which can render to our surface
        compatible_surface: Some(&surface),
    }).await.unwrap();

    // Create the logical device and command queue
    let (device, queue) = adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
        },
        None,
    ).await.expect("Failed to create WGPU device");

    (adapter, Arc::new(device), Arc::new(queue))
}

fn swapchain_desc(width: u32, height: u32) -> wgpu::SwapChainDescriptor {
    wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width,
        height,
        present_mode: wgpu::PresentMode::Mailbox,
    }
}
