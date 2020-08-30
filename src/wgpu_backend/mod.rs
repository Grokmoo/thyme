use wgpu::{Device, Queue, Surface, TextureFormat, include_spirv};

use crate::{Renderer, Point};
/**
A Thyme [`Renderer`](trait.Renderer.html) for [`wgpu`](https://github.com/gfx-rs/wgpu-rs).

This adapter registers image and font data as textures, and renders each frame.

Note that the SPIRV shaders are manually built using [`shaderc`](https://github.com/google/shaderc).
The commands should roughly be:
```bash
glslc -fshader-stage=vertex -fentry-point=main -o vert.spirv thyme\src\wgpu_backend\shaders\vert.glsl
glslc -fshader-stage=fragment -fentry-point=main -o frag.spirv thyme\src\wgpu_backend\shaders\frag.glsl
```
**/
pub struct WgpuRenderer {

}

impl WgpuRenderer {
    pub fn new(device: &Device, queue: &Queue) -> WgpuRenderer {
        let vert_shader = device.create_shader_module(include_spirv!("shaders/vert.spirv"));
        let frag_shader = device.create_shader_module(include_spirv!("shaders/frag.spirv"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vert_shader,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &frag_shader,
                entry_point: "main",
            }),
            // Use the default rasterizer state: no culling, no depth bias
            rasterization_state: None,
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[TextureFormat::Rgba8Sint.into()],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

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