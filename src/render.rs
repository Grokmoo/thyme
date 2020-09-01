use std::num::NonZeroU16;

use crate::{Color, Rect, Point, Error};
use crate::font::{FontSource, Font};

/// A trait to be implemented on the type to be used for Event handling.  See [`WinitIO`](struct.WinitIO.html)
/// for an example implementation.  The IO handles events from an external source and passes them to the Thyme
/// [`Context`](struct.Context.html).
pub trait IO {
    /// Returns the current window scale factor (1.0 for logical pixel size = physical pixel size).
    fn scale_factor(&self) -> f32;

    /// Returns the current window size in logical pixels.
    fn display_size(&self) -> Point;
}

/// A trait to be implemented on the type to be used for rendering the UI.  See [`GliumRenderer`](struct.GliumRenderer.html)
/// for an example implementation.  The `Renderer` takes a completed frame and renders the widget tree stored within it.
pub trait Renderer {
    /// Register a font with Thyme.  This method is called via the [`ContextBuilder`](struct.ContextBuilder.html).
    fn register_font(
        &mut self,
        handle: FontHandle,
        source: &FontSource,
        size: f32,
        scale: f32,
    ) -> Result<Font, Error>;

    /// Register a texture with Thyme.  This method is called via the [`ContextBuilder`](struct.ContextBuilder.html).
    fn register_texture(
        &mut self,
        handle: TextureHandle,
        image_data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<TextureData, Error>;
}

pub(crate) fn view_matrix(display_pos: Point, display_size: Point) -> [[f32; 4]; 4] {
    let left = display_pos.x;
    let right = display_pos.x + display_size.x;
    let top = display_pos.y;
    let bot = display_pos.y + display_size.y;

    [
        [         (2.0 / (right - left)),                             0.0,  0.0, 0.0],
        [                            0.0,          (2.0 / (top - bot)),  0.0, 0.0],
        [                            0.0,                             0.0, -1.0, 0.0],
        [(right + left) / (left - right), (top + bot) / (bot - top),  0.0, 1.0],
    ]
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DrawMode {
    Image(TextureHandle),
    Font(FontHandle),
}

pub trait DrawList {
    fn push_rect(
        &mut self,
        pos: [f32; 2],
        size: [f32; 2],
        tex: [TexCoord; 2],
        color: Color,
        clip: Rect,
    );

    /// the number of vertices currently contained in this list
    fn len(&self) -> usize;

    /// adjust the positions of all vertices from the last one in the list
    /// to the one at the specified `since_index`, by the specified `amount`
    fn back_adjust_positions(&mut self, since_index: usize, amount: Point);
}

/// An implementation of DrawList that does nothing.  It should be (mostly) optimized
/// out when used
pub(crate) struct DummyDrawList {
    index: usize,
}

impl DummyDrawList {
    pub fn new() -> DummyDrawList {
        DummyDrawList { index: 0 }
    }
}

impl DrawList for DummyDrawList {
    fn push_rect(
        &mut self,
        _pos: [f32; 2],
        _size: [f32; 2],
        _tex: [TexCoord; 2],
        _color: Color,
        _clip: Rect,
    ) {
        self.index += 1;
    }

    fn len(&self) -> usize { self.index }

    fn back_adjust_positions(&mut self, _since_index: usize, _amount: Point) {}
}

pub struct TextureData {
    handle: TextureHandle,
    size: [u32; 2],
}

impl TextureData {
    pub fn new(handle: TextureHandle, width: u32, height: u32) -> TextureData {
        TextureData {
            handle,
            size: [width, height],
        }
    }

    pub fn tex_coord(&self, x: u32, y: u32) -> TexCoord {
        let x = x as f32 / self.size[0] as f32;
        let y = y as f32 / self.size[1] as f32;
        TexCoord([x, y])
    }

    pub fn handle(&self) -> TextureHandle { self.handle }
}

#[derive(Copy, Clone)]
pub struct TexCoord([f32; 2]);

impl TexCoord {
    pub fn new(x: f32, y: f32) -> TexCoord {
        TexCoord([x, y])
    }

    pub fn x(&self) -> f32 { self.0[0] }
    pub fn y(&self) -> f32 { self.0[1] }
}

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    id: NonZeroU16,
}

impl Default for TextureHandle {
    fn default() -> Self {
        TextureHandle { id: NonZeroU16::new(1).unwrap() }
    }
}

impl TextureHandle {
    pub fn id(self) -> usize { (self.id.get() - 1).into() }

    pub fn next(self) -> TextureHandle {
        if self.id.get() == u16::MAX {
            panic!("Cannot allocate more than {} textures", u16::MAX);
        }

        TextureHandle {
            id: NonZeroU16::new(self.id.get() + 1).unwrap()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle {
    id: NonZeroU16,
}

impl Default for FontHandle {
    fn default() -> Self {
        FontHandle { id: NonZeroU16::new(1).unwrap() }
    }
}

impl FontHandle {
    pub fn id(self) -> usize { (self.id.get() - 1).into() }

    pub fn next(self) -> FontHandle {
        if self.id.get() == u16::MAX {
            panic!("Cannot allocate more than {} fonts", u16::MAX);
        }
        FontHandle {
            id: NonZeroU16::new(self.id.get() + 1).unwrap()
        }
    }
}