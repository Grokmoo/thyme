use std::collections::{HashMap, HashSet};

mod frame;
pub use frame::{Frame, Vertex, DrawMode, DrawList, DrawData};

mod glium_backend;
pub use glium_backend::GliumRenderer;

mod theme;
pub use theme::{ThemeSet, Image, FontChar, Font, FontSummary};

mod theme_definition;
pub use theme_definition::{ThemeDefinition, AnimStateKey, AnimState, Align, Color, Layout, WidthRelative, HeightRelative};

mod point;
pub use point::{Point, Rect, Border};

mod widget;
pub use widget::{Widget, WidgetBuilder};

mod winit_io;
pub use winit_io::WinitIo;

pub trait IO {}

pub trait Renderer {
    fn register_font(
        &mut self,
        handle: FontHandle,
        source: &FontSource,
        size: f32,
    ) -> Result<Font, Error>;

    fn register_texture(
        &mut self,
        handle: TextureHandle,
        image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>
    ) -> Result<TextureData, Error>;
}

pub struct Builder<'a, R: Renderer, I: IO> {
    renderer: &'a mut R,
    _io: &'a mut I,
    font_sources: HashMap<String, FontSource>,
    textures: HashMap<String, TextureData>,
    next_texture_handle: TextureHandle,
    theme_def: ThemeDefinition,
}

impl<'a, R: Renderer, I: IO> Builder<'a, R, I> {
    pub fn new<T: serde::Deserializer<'a>>(theme: T, renderer: &'a mut R, io: &'a mut I) -> Result<Builder<'a, R, I>, T::Error> {
        let theme_def: ThemeDefinition = serde::Deserialize::deserialize(theme)?;

        Ok(Builder {
            renderer,
            _io: io,
            font_sources: HashMap::new(),
            textures: HashMap::new(),
            next_texture_handle: TextureHandle::default(),
            theme_def,
        })
    }

    pub fn register_font_source<T: Into<String>>(&mut self, id: T, data: Vec<u8>) -> Result<(), Error> {
        let font = match rusttype::Font::try_from_vec(data) {
            Some(font) => font,
            None => return Err(
                Error::FontSource(format!("Unable to parse '{}' as ttf", id.into()))
            )
        };
        self.font_sources.insert(id.into(), FontSource { font });

        Ok(())
    }

    pub fn register_texture<T: Into<String>>(
        &mut self,
        id: T,
        image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>
    ) -> Result<(), Error> {
        let handle = self.next_texture_handle;
        let data = self.renderer.register_texture(handle, image)?;
        self.textures.insert(id.into(), data);
        self.next_texture_handle.id += 1;

        Ok(())
    }

    pub fn build(self, display_size: Point) -> Result<Context, Error> {
        let textures = self.textures;
        let fonts = self.font_sources;
        let themes = ThemeSet::new(self.theme_def, textures, fonts, self.renderer)?;
        Ok(Context {
            display_size,
            themes,
            opened: HashSet::new(),
            mouse_pos: Point::default(),
            mouse_pressed: [false; 3],
            mouse_clicked: [false; 3],
            mouse_taken_last_frame: false,
            mouse_pressed_outside: [false; 3],
        })
    }
}

pub struct Context {
    themes: ThemeSet,
    mouse_taken_last_frame: bool,

    mouse_pressed_outside: [bool; 3],

    opened: HashSet<String>,

    mouse_pos: Point,
    mouse_pressed: [bool; 3],
    mouse_clicked: [bool; 3],
    display_size: Point,
}

impl Context {
    fn mouse_pressed_outside(&self) -> bool {
        for pressed in self.mouse_pressed_outside.iter() {
            if *pressed { return true; }
        }
        false
    }

    pub(crate) fn set_display_size(&mut self, size: Point) {
        self.display_size = size;
    }

    pub(crate) fn set_mouse_pressed(&mut self, pressed: bool, index: usize) {
        if index >= self.mouse_pressed.len() {
            return;
        }

        // don't take a mouse press that started outside the GUI elements
        if pressed && !self.mouse_taken_last_frame {
            self.mouse_pressed_outside[index] = true;
        }

        if !pressed && self.mouse_pressed_outside[index] {
            self.mouse_pressed_outside[index] = false;
        }

        if self.mouse_pressed[index] && !pressed {
            self.mouse_clicked[index] = true;
        }

        self.mouse_pressed[index] = pressed;
    }

    pub(crate) fn set_mouse_pos(&mut self, pos: Point) {
        self.mouse_pos = pos;
    }

    pub fn wants_mouse(&self) -> bool { self.mouse_taken_last_frame }

    pub fn create_frame(&mut self) -> (Frame, Widget) {
        let theme = self.themes.handle("root").unwrap();
        let display_size = self.display_size;

        let frame = Frame::new(display_size, self);
        let root = Widget::root(theme, display_size);

        (frame, root)
    }
}

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

pub struct FontSource {
    font: rusttype::Font<'static>,
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle {
    id: usize,
}

pub struct TextureData {
    handle: TextureHandle,
    size: [u32; 2],
}

impl TextureData {
    fn tex_coord(&self, x: u32, y: u32) -> TexCoord {
        let x = x as f32 / self.size[0] as f32;
        let y = 1.0 - y as f32 / self.size[1] as f32;
        TexCoord([x, y])
    }
}

#[derive(Copy, Clone)]
pub(crate) struct TexCoord([f32; 2]);

impl Default for TexCoord {
    fn default() -> TexCoord {
        TexCoord([0.0, 0.0])
    }
}

impl From<TexCoord> for [f32; 2] {
    fn from(coord: TexCoord) -> Self {
        coord.0
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    id: usize,
}