use std::num::NonZeroU16;

use crate::{Color, Clip, Point, Error};
use crate::font::{FontSource, Font};

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

#[derive(Copy, Clone, PartialEq, Eq)]
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
        clip: Clip,
    );

    /// the number of vertices currently contained in this list
    fn len(&self) -> usize;

    /// adjust the positions of all vertices from the last one in the list
    /// to the one at the specified `since_index`, by the specified `amount`
    fn back_adjust_positions(&mut self, since_index: usize, amount: Point);
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
        let y = 1.0 - y as f32 / self.size[1] as f32;
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