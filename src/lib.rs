use std::collections::{HashMap};
use std::rc::Rc;
use std::cell::RefCell;

mod font;
pub use font::{Font, FontChar, FontSummary, FontSource, FontHandle};

mod frame;
pub use frame::{Frame};

mod glium_backend;
pub use glium_backend::GliumRenderer;

mod image;
pub use crate::image::{Image};

mod theme;
pub use theme::{ThemeSet};

mod theme_definition;
pub use theme_definition::{ThemeDefinition, AnimStateKey, AnimState, Align, Color, Layout, WidthRelative, HeightRelative};

mod point;
pub use point::{Clip, Point, Rect, Border};

mod recipes;

mod render;
pub use render::{Vertex, DrawMode, DrawList, DrawData};

mod widget;
pub use widget::{Widget, WidgetBuilder, WidgetState};

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
        image_data: &[u8],
        dimensions: (u32, u32),
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

    /// Registers the font data for use with Thyme via the specified `id`.  The `data` must consist
    /// of the full binary for a valid TTF or OTF file.
    /// Once the font has been registered, it can be accessed in your theme file via the font `source`.
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

    /// Registers the image data for use with Thyme via the specified `id`.  The `data` must consist of
    /// raw binary image data in RGBA format, with 4 bytes per pixel.  The data must start at the
    /// bottom-left hand corner pixel and progress left-to-right and bottom-to-top.  `data.len()` must
    /// equal `dimensions.0 * dimensions.1 * 4`
    /// Once the image has been registered, it can be accessed in your theme file via the image `source`.
    pub fn register_texture<T: Into<String>>(
        &mut self,
        id: T,
        data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<(), Error> {
        let handle = self.next_texture_handle;
        let data = self.renderer.register_texture(handle, data, dimensions)?;
        self.textures.insert(id.into(), data);
        self.next_texture_handle.id += 1;

        Ok(())
    }

    pub fn build(self, display_size: Point) -> Result<Context, Error> {
        let textures = self.textures;
        let fonts = self.font_sources;
        let themes = ThemeSet::new(self.theme_def, textures, fonts, self.renderer)?;
        Ok(Context::new(themes, display_size))
    }
}

#[derive(Copy, Clone)]
pub struct PersistentState {
    pub is_open: bool,
    pub resize: Point,
    pub moved: Point,
}

impl Default for PersistentState {
    fn default() -> Self {
        PersistentState {
            is_open: true,
            resize: Point::default(),
            moved: Point::default(),
        }
    }
}

pub(crate) struct ContextInternal {
    themes: ThemeSet,
    mouse_taken_last_frame: Option<String>,

    mouse_pressed_outside: [bool; 3],

    persistent_state: HashMap<String, PersistentState>,

    last_mouse_pos: Point,
    mouse_pos: Point,
    mouse_pressed: [bool; 3],
    mouse_clicked: [bool; 3],
    display_size: Point,
}

impl ContextInternal {
    fn init_state<T: Into<String>>(&mut self, id: T, open: bool) {
        self.persistent_state.entry(id.into()).or_insert(
            PersistentState {
                is_open: open,
                ..Default::default()
            }
        );
    }

    fn state(&self, id: &str) -> PersistentState {
        self.persistent_state.get(id).copied().unwrap_or_default()
    }

    fn state_mut<T: Into<String>>(&mut self, id: T) -> &mut PersistentState {
        self.persistent_state.entry(id.into()).or_default()
    }

    fn mouse_pressed_outside(&self) -> bool {
        for pressed in self.mouse_pressed_outside.iter() {
            if *pressed { return true; }
        }
        false
    }
}

pub struct Context {
    internal: Rc<RefCell<ContextInternal>>,
}

impl Context {
    fn new(themes: ThemeSet, display_size: Point) -> Context {
        let internal = ContextInternal {
            display_size,
            themes,
            persistent_state: HashMap::new(),
            mouse_pos: Point::default(),
            last_mouse_pos: Point::default(),
            mouse_pressed: [false; 3],
            mouse_clicked: [false; 3],
            mouse_taken_last_frame: None,
            mouse_pressed_outside: [false; 3],
        };

        Context {
            internal: Rc::new(RefCell::new(internal))
        }
    }

    pub fn wants_mouse(&self) -> bool {
        let internal = self.internal.borrow();
        internal.mouse_taken_last_frame.is_some()
    }

    pub(crate) fn set_display_size(&mut self, size: Point) {
        let mut internal = self.internal.borrow_mut();
        internal.display_size = size;
    }

    pub(crate) fn set_mouse_pressed(&mut self, pressed: bool, index: usize) {
        let mut internal = self.internal.borrow_mut();

        if index >= internal.mouse_pressed.len() {
            return;
        }

        // don't take a mouse press that started outside the GUI elements
        if pressed && internal.mouse_taken_last_frame.is_none() {
            internal.mouse_pressed_outside[index] = true;
        }

        if !pressed && internal.mouse_pressed_outside[index] {
            internal.mouse_pressed_outside[index] = false;
        }

        if internal.mouse_pressed[index] && !pressed {
            internal.mouse_clicked[index] = true;
        }

        internal.mouse_pressed[index] = pressed;
    }

    pub(crate) fn set_mouse_pos(&mut self, pos: Point) {
        let mut internal = self.internal.borrow_mut();
        internal.mouse_pos = pos;
    }

    pub fn create_frame(&mut self) -> Frame {
        let (theme, display_size) = {
            let internal = self.internal.borrow();
            (internal.themes.handle("root").unwrap(), internal.display_size)
        };
        let context = Context { internal: Rc::clone(&self.internal) };

        let root = Widget::root(theme, display_size);
        Frame::new(context, root)
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