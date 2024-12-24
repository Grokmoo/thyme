use std::path::PathBuf;

use glium::glutin::surface::WindowSurface;
use glutin::surface::GlSurface;
use winit::{application::ApplicationHandler, error::EventLoopError};

use crate::{Error, Point, BuildOptions, ContextBuilder, Context, WinitIo, Frame};

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
    options: BuildOptions,
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
            options: BuildOptions::default(),
        }
    }

    /// Set the time in milliseconds for tooltips to show.
    /// See [`BuildOptions`](struct.BuildOptions.html)
    pub fn with_tooltip_time(mut self, time_millis: u32) -> AppBuilder {
        self.options.tooltip_time = time_millis;
        self
    }

    /// Set the number of lines that scrollbars will scroll per mouse scroll.
    /// See [`BuildOptions`](struct.BuildOptions.html)
    pub fn with_line_scroll(mut self, line_scroll: f32) -> AppBuilder {
        self.options.line_scroll = line_scroll;
        self
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
        use std::ffi::CString;
        use std::num::NonZeroU32;

        use glutin::prelude::*;
        use glutin::config::ConfigTemplateBuilder;
        use glutin::context::{ContextApi, ContextAttributesBuilder, Version};
        use glutin_winit::DisplayBuilder;
        use glutin::display::GetGlDisplay;
        use winit::raw_window_handle::HasWindowHandle;
        use glium::backend::glutin::simple_window_builder::GliumEventLoop;
        use winit::window::Window;

        use crate::winit_io::WinitError;
        use crate::GlError;

        const OPENGL_MAJOR_VERSION: u8 = 3;
        const OPENGL_MINOR_VERSION: u8 = 2;

        if self.logger {
            crate::log::init(log::Level::Warn).unwrap();
        }

        let event_loop = glium::winit::event_loop::EventLoop::builder()
        .build().map_err(|e| Error::Winit(WinitError::EventLoop(e)))?;

        let attrs = Window::default_attributes()
            .with_title("Simple Glium Window")
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 480));

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(attrs));
        let config_template_builder = ConfigTemplateBuilder::new();
        let (window, gl_config) = event_loop.build(display_builder, config_template_builder, |mut configs| {
                configs.next().unwrap()
            })
            .unwrap();
        let window = window.unwrap();

        let window_handle = window.window_handle().map_err(|e| Error::Winit(WinitError::HandleError(e)))?;
        let raw_window_handle = window_handle.as_raw();

        // Now we get the window size to use as the initial size of the Surface
        let (width, height): (u32, u32) = window.inner_size().into();
        let attrs =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(
                    raw_window_handle,
                    NonZeroU32::new(width).unwrap(),
                    NonZeroU32::new(height).unwrap(),
                );

        let surface = unsafe {
            gl_config.display().create_window_surface(&gl_config, &attrs).unwrap()
        };

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(OPENGL_MAJOR_VERSION, OPENGL_MINOR_VERSION))))
            .build(Some(raw_window_handle));

        let windowed_context = unsafe {
            gl_config.display().create_context(&gl_config, &context_attributes).map_err(GlError::Glutin).map_err(Error::Gl)?
        };

        let display_context = windowed_context.make_current(&surface).map_err(GlError::Glutin).map_err(Error::Gl)?;

        {
            let gl_context = display_context.display();
            gl::load_with(|ptr| {
                let c_str = CString::new(ptr).unwrap();
                gl_context.get_proc_address(&c_str) as *const _
            })

        }

        let mut io = crate::WinitIo::new(&window, self.window_size)
            .map_err(Error::Winit)?;
        let mut renderer = crate::GLRenderer::new();
        let mut context_builder = crate::ContextBuilder::new(self.options.clone());

        self.register_resources(&mut context_builder)?;

        let context = context_builder.build(&mut renderer, &mut io)?;

        Ok(GlApp { io, renderer, context, event_loop, window, surface, display_context })
    }
    
    /// Creates a [`GliumApp`](struct.GliumApp.html) object, setting up Thyme as specified
    /// in this Builder and using the [`GliumRenderer`](struct.GliumRenderer.html).
    #[cfg(feature="glium_backend")]
    pub fn build_glium(self) -> Result<GliumApp, Error> {
        use crate::winit_io::WinitError;

        if self.logger {
            crate::log::init(log::Level::Warn).unwrap();
        }

        let event_loop = glium::winit::event_loop::EventLoop::builder()
            .build().map_err(|e| Error::Winit(WinitError::EventLoop(e)))?;

        let (window, display) = glium::backend::glutin::SimpleWindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(self.window_size.x as u32, self.window_size.y as u32)
            .build(&event_loop);

        let mut io = crate::WinitIo::new(&window, self.window_size)
            .map_err(Error::Winit)?;
        let mut renderer = crate::GliumRenderer::new(&display)
            .map_err(Error::Glium)?;
        let mut context_builder = crate::ContextBuilder::new(self.options.clone());

        self.register_resources(&mut context_builder)?;

        let context = context_builder.build(&mut renderer, &mut io)?;

        Ok(GliumApp { io, renderer, context, display, window, event_loop })
    }

    fn register_resources(&self, context_builder: &mut ContextBuilder) -> Result<(), Error> {
        let theme_src = match self.themes.as_ref() {
            None => return Err(Error::Theme("No theme files specified".to_string())),
            Some(src) => src,
        };

        let theme_files = theme_src.get_files(self.base_dir.clone(), &["yml", "yaml"])?;
        let theme_paths: Vec<_> = theme_files.iter().map(|(_, path)| path.as_path()).collect();

        context_builder.register_theme_from_files(&theme_paths)?;

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

    /// The OpenGL / Glutin window
    pub window: winit::window::Window,

    /// the window surface for drawing
    pub surface: glutin::surface::Surface<WindowSurface>,

    /// the GL display context
    pub display_context: glutin::context::PossiblyCurrentContext,
}

#[cfg(feature="gl_backend")]
struct GlAppRunner<F: Fn(&mut Frame)> {
    io: WinitIo,
    renderer: crate::GLRenderer,
    context: Context,
    window: winit::window::Window,
    surface: glutin::surface::Surface<WindowSurface>,
    display_context: glutin::context::PossiblyCurrentContext,
    f: F,
}

#[cfg(feature="gl_backend")]
impl<F: Fn(&mut Frame)> ApplicationHandler for GlAppRunner<F> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) { }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent;
        match event {
            WindowEvent::RedrawRequested => {
                self.renderer.clear_color(0.0, 0.0, 0.0, 0.0);

                let mut ui = self.context.create_frame();
    
                (self.f)(&mut ui);
    
                self.renderer.draw_frame(ui);

                self.surface.swap_buffers(&self.display_context).unwrap();
    
                // Was:
                // renderer.clear_color(0.0, 0.0, 0.0, 1.0);
                // let mut ui = context.create_frame();
                // (f)(&mut ui);
                // renderer.draw_frame(ui);
                // windowed_context.swap_buffers().unwrap();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => {
                self.io.handle_event(&mut self.context, &event);
            }
        }
    }
}

#[cfg(feature="gl_backend")]
impl GlApp {
    /// Runs the Winit main loop for this app
    pub fn main_loop<F: Fn(&mut Frame) + 'static>(self, f: F) -> Result<(), EventLoopError> {
        let mut runner = GlAppRunner {
            io: self.io,
            renderer: self.renderer,
            context: self.context,
            window: self.window,
            surface: self.surface,
            display_context: self.display_context,
            f,
        };

        self.event_loop.run_app(&mut runner)
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

    /// The Winit Window
    pub window: winit::window::Window,

    /// The Glium / Winit Display
    pub display: glium::Display<WindowSurface>,

    /// The Glium / Winit Event loop
    pub event_loop: winit::event_loop::EventLoop<()>,
}

#[cfg(feature="glium_backend")]
impl GliumApp {
    /// Runs the Winit main loop for this app
    pub fn main_loop<F: Fn(&mut Frame) + 'static>(self, f: F) -> Result<(), EventLoopError> {
        let mut runner = GliumAppRunner {
            io: self.io,
            renderer: self.renderer,
            context: self.context,
            display: self.display,
            window: self.window,
            f,
        };
        
        self.event_loop.run_app(&mut runner)
    }
}

#[cfg(feature="glium_backend")]
struct GliumAppRunner<F: Fn(&mut Frame)> {
    pub io: WinitIo,
    pub renderer: crate::GliumRenderer,
    pub context: Context,
    pub display: glium::Display<WindowSurface>,
    pub window: winit::window::Window,
    pub f: F,
}

#[cfg(feature="glium_backend")]
impl<F: Fn(&mut Frame)> ApplicationHandler for GliumAppRunner<F> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) { }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use glium::Surface;
        use winit::event::WindowEvent;
        match event {
            WindowEvent::RedrawRequested => {
                let mut target = self.display.draw();
                target.clear_color(0.0, 0.0, 0.0, 0.0);
    
                let mut ui = self.context.create_frame();
    
                (self.f)(&mut ui);
    
                self.renderer.draw_frame(&mut target, ui).unwrap();
    
                target.finish().unwrap();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => {
                self.io.handle_event(&mut self.context, &event);
            }
        }
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