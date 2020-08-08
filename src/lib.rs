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
mod widget;
mod winit_io;

pub use frame::Frame;
pub use point::{Clip, Point, Border};
pub use widget::{WidgetBuilder, WidgetState};
pub use context::{Context, ContextBuilder, PersistentState};
pub use theme_definition::{AnimStateKey, AnimState, Align, Color, Layout, WidthRelative, HeightRelative};
pub use winit_io::WinitIo;
pub use glium_backend::GliumRenderer;

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