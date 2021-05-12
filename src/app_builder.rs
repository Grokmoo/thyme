use std::path::{PathBuf};

#[cfg(feature="wgpu_backend")]
use std::sync::Arc;

use crate::{Error, Point, ContextBuilder, Context, WinitIo, Frame};

/// An easy to use but still fairly configurable builder, allowing you to get
/// a Thyme app up in just a few lines of code.  It is designed to cover the
/// majority of cases and handles display creation, asset loading, and
/// initial Thyme setup.  If your use case isn't covered here, you'll need to
/// manually create your [`ContextBuilder`](struct.ContextBuilder.html), and
/// associated structs.  See the examples.
pub struct AppBuilder {
    title: String,
    window_size: Point,
    themes: Option<AssetSource>,
    fonts: Option<AssetSource>,
    images: Option<AssetSource>,
    base_dir: PathBuf,
    logger: bool,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppBuilder {
    /**
    Creates a new empty App builder.  Use with the builder pattern.
    
    # Example
    // Assuming you have a theme definition in theme.yml, fonts in the `fonts`
    // directory and images in the `images` directory:
    let app = AppBuilder::new()
        .with_title("My App")
        .with_window_size(1600.0, 900.0)
        .with_theme_file("theme.yml")
        .with_font_dir("fonts")
        .with_image_dir("images")
        .build_glium();
    **/
    pub fn new() -> AppBuilder {
        AppBuilder {
            title: "Thyme App".to_string(),
            window_size: Point::new(1280.0, 720.0),
            base_dir: PathBuf::new(),
            themes: None,
            fonts: None,
            images: None,
            logger: false,
        }
    }

    /// If called, this App Builder will setup a default Thyme logger
    /// at the warn level.  See [`SimpleLogger`](struct.SimpleLogger.html).
    pub fn with_logger(mut self) -> AppBuilder {
        self.logger = true;
        self
    }

    /// Specifies the window title for this app.
    pub fn with_title<T: Into<String>>(mut self, title: T) -> AppBuilder {
        self.title = title.into();
        self
    }

    /// Specifies the window size, in logical pixels, for this app.
    pub fn with_window_size(mut self, x: f32, y: f32) -> AppBuilder {
        self.window_size = Point::new(x, y);
        self
    }

    /// Specifies a top level base directory, that all other assets will be
    /// read as subdirectories of.  By default, this will just be the current
    /// working directory
    pub fn with_base_dir(mut self, dir: &str) -> AppBuilder {
        self.base_dir = PathBuf::from(dir);
        self
    }

    /// Specifies the set of YAML theme files to read in as your [`theme`](index.html) definition.  The filename
    /// is relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_theme_files`](#method.with_theme_files), [`with_theme_file`](#method.with_theme_file), or
    /// [`with_theme_dir`](#method.with_theme_dir) will take effect.
    pub fn with_theme_files(mut self, files: &[&str]) -> AppBuilder {
        self.themes = Some(AssetSource::Files(files.iter().map(PathBuf::from).collect()));
        self
    }

    /// Specifies a single YAML theme file to read in as your [`theme`](index.html) definition.  The filename
    /// is relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_theme_files`](#method.with_theme_files), [`with_theme_file`](#method.with_theme_file), or
    /// [`with_theme_dir`](#method.with_theme_dir) will take effect.
    pub fn with_theme_file(mut self, file: &str) -> AppBuilder {
        self.themes = Some(AssetSource::Files(vec![PathBuf::from(file)]));
        self
    }

    /// Specifies to read all YAML files inside the specified directory and parse them to create your
    /// [`theme`](index.html) definition.  The `dir` path is relative to the
    /// [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_theme_files`](#method.with_theme_files), [`with_theme_file`](#method.with_theme_file), or
    /// [`with_theme_dir`](#method.with_theme_dir) will take effect.
    pub fn with_theme_dir(mut self, dir: &str) -> AppBuilder {
        self.themes = Some(AssetSource::Directory(PathBuf::from(dir)));
        self
    }

    /// Specifies to read the specified TTF files as fonts for use in your [`theme`](index.html).  The fonts
    /// will be registered with an ID of the filestem (filename without extensions) to the Context,
    /// see [`ContextBuilder.register_font_from_file`](struct.ContextBuilder.html#register_font_from_file)
    /// The paths are relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_font_files`](#method.with_font_files), [`with_font_file`](#method.with_font_file), or
    /// [`with_font_dir`](#method.with_font_dir) will take effect.
    pub fn with_font_files(mut self, files: &[&str]) -> AppBuilder {
        self.fonts = Some(AssetSource::Files(files.iter().map(PathBuf::from).collect()));
        self
    }

    /// Specifies to read the specified single TTF file as a font for use in your [`theme`](index.html).  The font
    /// will be registered with an ID of the filestem (filename without extensions) to the Context,
    /// see [`ContextBuilder.register_font_from_file`](struct.ContextBuilder.html#register_font_from_file)
    /// The paths are relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_font_files`](#method.with_font_files), [`with_font_file`](#method.with_font_file), or
    /// [`with_font_dir`](#method.with_font_dir) will take effect.
    pub fn with_font_file(mut self, file: &str) -> AppBuilder {
        self.fonts = Some(AssetSource::Files(vec![PathBuf::from(file)]));
        self
    }

    /// Specifies to read all TTF files in the directory as fonts for use in your [`theme`](index.html).  The fonts
    /// will be registered with an ID of the filestem (filename without extensions) to the Context,
    /// see [`ContextBuilder.register_font_from_file`](struct.ContextBuilder.html#register_font_from_file)
    /// The paths are relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_font_files`](#method.with_font_files), [`with_font_file`](#method.with_font_file), or
    /// [`with_font_dir`](#method.with_font_dir) will take effect.
    pub fn with_font_dir(mut self, dir: &str) -> AppBuilder {
        self.fonts = Some(AssetSource::Directory(PathBuf::from(dir)));
        self
    }

    /// Specifies to read the `files` as images for use in your [`theme`](index.html).  The images
    /// will be registered with ID of the filestem (filename without extensions) to the Context,
    /// see [`ContextBuilder.register_texture_from_file`](struct.ContextBuilder.html#register_texture_from_file)
    /// The paths are relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_image_files`](#method.with_image_files), [`with_image_file`](#method.with_image_file), or
    /// [`with_image_dir`](#method.with_image_dir) will take effect.
    pub fn with_image_files(mut self, files: &[&str]) -> AppBuilder {
        self.images = Some(AssetSource::Files(files.iter().map(PathBuf::from).collect()));
        self
    }

    /// Specifies to read the file as a single image for use in your [`theme`](index.html).  The image
    /// will be registered with ID of the filestem (filename without extensions) to the Context,
    /// see [`ContextBuilder.register_texture_from_file`](struct.ContextBuilder.html#register_texture_from_file)
    /// The path is relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_image_files`](#method.with_image_files), [`with_image_file`](#method.with_image_file), or
    /// [`with_image_dir`](#method.with_image_dir) will take effect.
    pub fn with_image_file(mut self, file: &str) -> AppBuilder {
        self.images = Some(AssetSource::Files(vec![PathBuf::from(file)]));
        self
    }

    /// Specifies to read all png and jpg files in the specified directory  as images for use in your[`theme`](index.html).
    /// The images will be registered with ID of the filestem (filename without extensions) to the Context,
    /// see [`ContextBuilder.register_texture_from_file`](struct.ContextBuilder.html#register_texture_from_file)
    /// The paths are relative to the [`base directory`](#method.with_base_dir).  Only the last of
    /// [`with_image_files`](#method.with_image_files), [`with_image_file`](#method.with_image_file), or
    /// [`with_image_dir`](#method.with_image_dir) will take effect.
    pub fn with_image_dir(mut self, dir: &str) -> AppBuilder {
        self.images = Some(AssetSource::Directory(PathBuf::from(dir)));
        self
    }

    /// Creates a [`GlApp`](struct.GlApp.html) object, setting up Thyme as specified in this
    /// builder and using the [`GlRenderer`](struct.GlRenderer.html).
    #[cfg(feature="gl_backend")]
    pub fn build_gl(self) -> Result<GlApp, Error> {
        use glutin::event_loop::EventLoop;
        use crate::gl_backend::GlError;

        const OPENGL_MAJOR_VERSION: u8 = 3;
        const OPENGL_MINOR_VERSION: u8 = 2;

        if self.logger {
            crate::log::init(log::Level::Warn).unwrap();
        }

        let event_loop = EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(glutin::dpi::LogicalSize::new(self.window_size.x, self.window_size.y));

        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(
                glutin::Api::OpenGl,
                (OPENGL_MAJOR_VERSION, OPENGL_MINOR_VERSION),
            ))
            .build_windowed(window_builder, &event_loop)
            .map_err(GlError::GlutinCreation)
            .map_err(Error::Gl)?;

        let windowed_context = unsafe {
            windowed_context
                .make_current()
                .map_err(|(_context, e)| GlError::GlutinContext(e))
                .map_err(Error::Gl)?
        };

        let _gl = {
            let gl_context = windowed_context.context();
            gl::load_with(|ptr| gl_context.get_proc_address(ptr) as *const _)
        };

        let mut io = crate::WinitIo::new(&event_loop, self.window_size)
            .map_err(Error::Winit)?;
        let mut renderer = crate::GLRenderer::new();
        let mut context_builder = crate::ContextBuilder::with_defaults();

        self.register_resources(&mut context_builder)?;

        let context = context_builder.build(&mut renderer, &mut io)?;

        Ok(GlApp { io, renderer, context, event_loop, windowed_context })
    }
    
    /// Creates a [`GliumApp`](struct.GliumApp.html) object, setting up Thyme as specified
    /// in this Builder and using the [`GliumRenderer`](struct.GliumRenderer.html).
    #[cfg(feature="glium_backend")]
    pub fn build_glium(self) -> Result<GliumApp, Error> {
        use glium::glutin::{event_loop::{EventLoop}, window::WindowBuilder};
        use glium::{Display};
        use crate::glium_backend::GliumError;

        if self.logger {
            crate::log::init(log::Level::Warn).unwrap();
        }

        let event_loop = EventLoop::new();
        let context = glium::glutin::ContextBuilder::new();
        let builder = WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(glium::glutin::dpi::LogicalSize::new(self.window_size.x, self.window_size.y));
        let display = Display::new(builder, context, &event_loop).map_err(GliumError::DisplayCreation)
            .map_err(Error::Glium)?;

        let mut io = crate::WinitIo::new(&event_loop, self.window_size)
            .map_err(Error::Winit)?;
        let mut renderer = crate::GliumRenderer::new(&display)
            .map_err(Error::Glium)?;
        let mut context_builder = crate::ContextBuilder::with_defaults();

        self.register_resources(&mut context_builder)?;

        let context = context_builder.build(&mut renderer, &mut io)?;

        Ok(GliumApp { io, renderer, context, display, event_loop })
    }

    /// Creates a [`WgpuApp`](struct.WgpuApp.html) object, setting up Thyme as specified
    /// in this Builder and using the [`WgpuRenderer`](struct.WgpuRenderer.html).
    #[cfg(feature="wgpu_backend")]
    pub fn build_wgpu(self) -> Result<WgpuApp, Error> {
        use winit::{
            event_loop::EventLoop,
            window::WindowBuilder,
            dpi::LogicalSize
        };

        if self.logger {
            crate::log::init(log::Level::Warn).unwrap();
        }

        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(self.window_size.x, self.window_size.y))
            .build(&event_loop).map_err(crate::winit_io::WinitError::Os).map_err(Error::Winit)?;

        // setup WGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let (_adapter, device, queue) = futures::executor::block_on(setup_wgpu(&instance, &surface));
        let sc_desc = swapchain_desc(self.window_size.x as u32, self.window_size.y as u32);
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        // create thyme backend
        let mut io = crate::WinitIo::new(&event_loop, self.window_size).map_err(Error::Winit)?;
        let mut renderer = crate::WgpuRenderer::new(Arc::clone(&device), Arc::clone(&queue));
        let mut context_builder = crate::ContextBuilder::with_defaults();

        self.register_resources(&mut context_builder)?;

        let context = context_builder.build(&mut renderer, &mut io)?;

        Ok(WgpuApp { io, renderer, context, event_loop, window, surface, swap_chain, device, queue })
    }

    fn register_resources(&self, context_builder: &mut ContextBuilder) -> Result<(), Error> {
        let theme_src = match self.themes.as_ref() {
            None => return Err(Error::Theme("No theme files specified".to_string())),
            Some(src) => src,
        };

        let theme_files = theme_src.get_files(self.base_dir.clone(), &["yml", "yaml"])?;
        let theme_paths: Vec<_> = theme_files.iter().map(|(_, path)| path.as_path()).collect();

        context_builder.register_theme_from_files(&theme_paths, serde_yaml::from_str::<serde_yaml::Value>)?;

        let image_src = match self.images.as_ref() {
            None => return Err(Error::Theme("No image files specified".to_string())),
            Some(src) => src,
        };

        for (tag, path) in image_src.get_files(self.base_dir.clone(), &["jpg", "jpeg", "png"])? {
            context_builder.register_texture_from_file(&tag, path.as_path());
        }

        let font_src = match self.fonts.as_ref() {
            None => return Err(Error::Theme("No font files specified".to_string())),
            Some(src) => src,
        };

        for (tag, path) in font_src.get_files(self.base_dir.clone(), &["ttf", "otf"])? {
            context_builder.register_font_from_file(tag, path.as_path());
        }
        
        Ok(())
    }
}

/// The WgpuApp object, containing the Thyme [`Context`](struct.Context.html), [`Renderer`](struct.WgpuRenderer.html),
/// and [`IO`](struct.WinitIo.html).   You can manually use the public members of this struct, or use [`main_loop`](#method.main_loop)
/// for basic use cases.
#[cfg(feature="wgpu_backend")]
pub struct WgpuApp {
    /// The Thyme IO
    pub io: WinitIo,

    /// The Thyme Renderer
    pub renderer: crate::WgpuRenderer,

    /// The Thyme Context
    pub context: Context,

    /// Winit Event loop
    pub event_loop: winit::event_loop::EventLoop<()>,

    /// Winit window
    pub window: winit::window::Window,

    /// The Wgpu output surface
    pub surface: wgpu::Surface,

    /// Wgpu Swapchain
    pub swap_chain: wgpu::SwapChain,

    /// Wgpu output device
    pub device: Arc<wgpu::Device>,

    /// Wgpu output queue
    pub queue: Arc<wgpu::Queue>,
}

#[cfg(feature="wgpu_backend")]
impl WgpuApp {
    /// Runs the Winit main loop for this app
    pub fn main_loop<F: Fn(&mut Frame) + 'static>(self, f: F) -> ! {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::ControlFlow,
        };

        let event_loop = self.event_loop;
        let mut renderer = self.renderer;
        let mut io = self.io;
        let mut context = self.context;
        let window = self.window;
        let mut swap_chain = self.swap_chain;
        let queue = self.queue;
        let device = self.device;
        let surface = self.surface;

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::MainEventsCleared => {
                    let frame = swap_chain.get_current_frame().unwrap().output;
                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    
                    let mut ui = context.create_frame();
    
                    (f)(&mut ui);
    
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[wgpu::RenderPassColorAttachment {
                                view: &frame.view,
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
}

/// The GlApp object, containing the Thyme [`Context`](struct.Context.html), [`Renderer`](struct.GlRenderer.html), and
/// [`IO`](struct.WinitIo.html).  YOu can manually use the public members of this struct, or use [`main_loop`](#method.main_loop)
/// for basic use cases.
#[cfg(feature="gl_backend")]
pub struct GlApp {
    /// The Thyme IO
    pub io: WinitIo,

    /// The Thyme Renderer
    pub renderer: crate::GLRenderer,

    /// The Thyme Context
    pub context: Context,

    /// The OpenGL / Winit event loop
    pub event_loop: winit::event_loop::EventLoop<()>,

    /// The OpenGL / Glutin windowed context
    pub windowed_context: glutin::ContextWrapper<glutin::PossiblyCurrent, glutin::window::Window>,
}

#[cfg(feature="gl_backend")]
impl GlApp {
    /// Runs the Winit main loop for this app
    pub fn main_loop<F: Fn(&mut Frame) + 'static>(self, f: F) -> ! {
        use glutin::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow},
        };

        let event_loop = self.event_loop;
        let windowed_context = self.windowed_context;
        let mut context = self.context;
        let mut renderer = self.renderer;
        let mut io = self.io;

        event_loop.run(move |event, _, control_flow| match event {
            Event::MainEventsCleared => {
                renderer.clear_color(0.0, 0.0, 0.0, 1.0);
    
                let mut ui = context.create_frame();
    
                (f)(&mut ui);
    
                renderer.draw_frame(ui);
    
                windowed_context.swap_buffers().unwrap();
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            event => {
                io.handle_event(&mut context, &event);
            }
        })
    }
}

/// The GliumApp object, containing the Thyme [`Context`](struct.Context.html), [`Renderer`](struct.GliumRenderer.html),
/// and [`IO`](struct.WinitIo.html).  You can manually use the public members of this struct, or use [`main_loop`](#method.main_loop)
/// for basic use cases.
#[cfg(feature="glium_backend")]
pub struct GliumApp {
    /// The Thyme IO
    pub io: WinitIo,

    /// The Thyme Renderer
    pub renderer: crate::GliumRenderer,

    /// The Thyme Context
    pub context: Context,

    /// The Glium / Winit Display
    pub display: glium::Display,

    /// The Glium / Winit Event loop
    pub event_loop: winit::event_loop::EventLoop<()>,
}

#[cfg(feature="glium_backend")]
impl GliumApp {
    /// Runs the Winit main loop for this app
    pub fn main_loop<F: Fn(&mut Frame) + 'static>(self, f: F) -> ! {
        use glium::glutin::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow},
        };
        use glium::{Surface};

        let event_loop = self.event_loop;
        let display = self.display;
        let mut context = self.context;
        let mut renderer = self.renderer;
        let mut io = self.io;

        event_loop.run(move |event, _, control_flow| match event {
            Event::MainEventsCleared => {
                let mut target = display.draw();
                target.clear_color(0.0, 0.0, 0.0, 0.0);
    
                let mut ui = context.create_frame();
    
                (f)(&mut ui);
    
                renderer.draw_frame(&mut target, ui).unwrap();
    
                target.finish().unwrap();
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            event => {
                io.handle_event(&mut context, &event);
            }
        })
    }
}

enum AssetSource {
    Files(Vec<PathBuf>),
    Directory(PathBuf),
}

impl AssetSource {
    fn get_files(&self, base: PathBuf, extensions: &[&str]) -> Result<Vec<(String, PathBuf)>, Error> {
        let mut out = Vec::new();

        match self {
            AssetSource::Files(files) => {
                for file in files {
                    let mut path = base.clone();
                    path.push(file);
                    add_path(path, &mut out);
                }
            }, AssetSource::Directory(path) => {
                let mut dir_path = base;
                dir_path.push(path);

                for entry in dir_path.read_dir().map_err(Error::IO)? {
                    let entry = entry.map_err(Error::IO)?;

                    let path = entry.path();
                    if !path.is_file() { continue; }

                    let path_ext = path.extension().map(|ext| ext.to_string_lossy()).unwrap_or_default();
                    let mut valid = false;
                    for extension in extensions {
                        if *extension == path_ext {
                            valid = true;
                            break;
                        }
                    }

                    if valid {
                        add_path(path, &mut out);
                    }
                }
            }
        }

        Ok(out)
    }
}

fn add_path(path: PathBuf, out: &mut Vec<(String, PathBuf)>) {
    let stem = match path.file_stem().map(|s| s.to_string_lossy()) {
        None => return,
        Some(stem) => stem,
    };

    out.push((stem.to_string(), path));
}

#[cfg(feature="wgpu_backend")]
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

#[cfg(feature="wgpu_backend")]
fn swapchain_desc(width: u32, height: u32) -> wgpu::SwapChainDescriptor {
    wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8Unorm,
        width,
        height,
        present_mode: wgpu::PresentMode::Mailbox,
    }
}