use crate::Renderer;

pub struct WgpuRenderer {

}

impl WgpuRenderer {
    pub fn new() -> WgpuRenderer {
        WgpuRenderer {
            
        }
    }
}

impl Renderer for WgpuRenderer {
    fn register_font(
        &mut self,
        handle: crate::render::FontHandle,
        source: &crate::font::FontSource,
        size: f32,
        scale: f32,
    ) -> Result<crate::font::Font, crate::Error> {
        todo!()
    }

    fn register_texture(
        &mut self,
        handle: crate::render::TextureHandle,
        image_data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<crate::render::TextureData, crate::Error> {
        todo!()
    }
}