//! Thyme is a highly customizable, themable immediate mode GUI toolkit for Rust.
//!
//! It is designed to be performant and flexible enough for use both in prototyping and production games and applications.
//! Requiring a theme and image sources adds some additional startup cost compared to most other UI toolkits, however
//! the advantage is full flexibility and control over the ultimate appearance of your UI.
//!
//! To use Thyme, you need the core library, a renderer (one based on [Glium](https://github.com/glium/glium) is included),
//! and event handling support (one based on [winit](https://github.com/rust-windowing/winit) is included).
//! All thyme widgets are drawn using images, with the image data registered with the renderer, and then individual
//! widget components defined within that image within the theme file.  Likewise, fonts `ttf` fonts are registered with
//! the renderer and then individual fonts for use in your UI are defined in the theme file.
//! Widgets themselves can be defined fully in source code, with only some basic templates in the theme file, or
//! you can largely leave only logic in the source, with layout, alignment, etc defined in the theme file.
//! See [`Context`](struct.Context.html) for further discussion of the theme format.
//!
//! The best place to start is to look in the examples folder for some basic Thyme apps.  You can also copy the
//! sample theme, image, and font files as a starting point for your projects.
//!
//! In general, you first create the [`ContextBuilder`](struct.ContextBuilder.html) and register resources with it.
//! Once done, you [`build`](struct.ContextBuilder.html#method.build) the associated [`Context`](struct.Context.html).
//! At each frame of your app, you [`create a Thyme frame`](struct.Context.html#method.create_frame).  The
//! [`Frame`](struct.Frame.html) is then passed along through your UI building routines, and is used to create
//! [`WidgetBuilders`](struct.WidgetBuilder.html) and populate your Widget tree.

pub mod bench;

mod context;
mod font;
mod frame;
mod glium_backend;
mod image;
mod theme;
mod recipes;
mod render;
mod theme_definition;
mod point;
mod scrollpane;
mod widget;
mod window;
mod winit_io;

pub use frame::Frame;
pub use point::{Rect, Point, Border};
pub use widget::{WidgetBuilder, WidgetState};
pub use context::{Context, ContextBuilder, PersistentState};
pub use scrollpane::ScrollpaneBuilder;
pub use theme_definition::{AnimStateKey, AnimState, Align, Color, Layout, WidthRelative, HeightRelative};
pub use window::WindowBuilder;
pub use winit_io::WinitIo;
pub use glium_backend::GliumRenderer;
pub use render::{IO, Renderer};

/// A generic error that can come from a variety of internal sources.
#[derive(Debug, Clone)]
pub enum Error {
    Theme(String),
    FontSource(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::Error::*;
        match self {
            Theme(msg) => write!(f, "Error creating theme from theme definition: {}", msg),
            FontSource(msg) => write!(f, "Error reading font source: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::Error::*;
        match self {
            Theme(..) => None,
            FontSource(..) => None,
        }
    }
}