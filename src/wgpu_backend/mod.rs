use std::rc::Rc;

use wgpu::{
    Buffer, BufferDescriptor, BufferUsage, BufferAddress,
    ColorStateDescriptor, BlendDescriptor, BlendFactor, BlendOperation, ColorWrite,
    BindingResource, BindGroupLayout, BindGroupEntry, BindingType, BindGroupDescriptor, BindGroup, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    Device, Queue, RenderPipeline, RenderPass,
    TextureFormat, TextureComponentType, TextureViewDimension, TextureDataLayout, TextureCopyView, TextureViewDescriptor,
    SamplerDescriptor, AddressMode, FilterMode,
    VertexBufferDescriptor, InputStepMode, vertex_attr_array,
    include_spirv,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::render::{DrawMode, view_matrix, TextureData, TexCoord, DrawList};
use crate::font::FontTextureWriter;
use crate::image::ImageDrawParams;
use crate::{Renderer, Frame, Point, Color, Rect};

/**
A Thyme [`Renderer`](trait.Renderer.html) for [`wgpu`](https://github.com/gfx-rs/wgpu-rs).

The adapter registers image and font data as textures, and renders each frame.

This renderer is implemented fairly naively at present and there is definitely room for optimization.
However, it is nonetheless already quite fast.
**/
pub struct WgpuRenderer {
    device: Rc<Device>,
    queue: Rc<Queue>,

    image_pipe: RenderPipeline,
    font_pipe: RenderPipeline,

    view_matrix_buffer: Buffer,
    view_matrix_bind_group: BindGroup,

    texture_layout: BindGroupLayout,

    textures: Vec<Texture>,
    fonts: Vec<Texture>,

    draw_list: WgpuDrawList,

    groups: Vec<BufferedGroup>,
}

impl WgpuRenderer {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> WgpuRenderer {
        /*
        Note that the SPIRV shaders are manually built using [`shaderc`](https://github.com/google/shaderc).
        This is slightly inconvenient, but I have found configuring shaders to compile at build time reliably
        in different environments too difficult.

        The commands to compile the shaders should be:
        ```bash
        cd src/wgpu_backend/shaders
        glslc -fshader-stage=vertex -fentry-point=main -o vert.spirv vert.glsl
        glslc -fshader-stage=fragment -fentry-point=main -o frag.spirv frag.glsl
        glslc -fshader-stage=fragment -fentry-point=main -o frag_font.spirv frag_font.glsl
        ```
        */
        let vert_shader = device.create_shader_module(include_spirv!("shaders/vert.spirv"));
        let frag_shader = device.create_shader_module(include_spirv!("shaders/frag.spirv"));
        let frag_font_shader = device.create_shader_module(include_spirv!("shaders/frag_font.spirv"));

        // setup the view matrix
        let view_matrix_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("view matrix buffer"),
            size: 64, // 4 x 4 x 4 bytes
            usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let view_matrix_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: BindingType::UniformBuffer {
                    dynamic: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let view_matrix_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("view matrix bind group"),
            layout: &view_matrix_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(view_matrix_buffer.slice(..)),
            }],
        });

        // setup the texture layout
        let texture_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("thyme texture layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: BindingType::SampledTexture {
                        multisampled: false,
                        component_type: TextureComponentType::Float,
                        dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("thyme pipeline layout"),
            bind_group_layouts: &[&view_matrix_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let mut pipe_desc = wgpu::RenderPipelineDescriptor {
            label: Some("thyme render pipeline"),
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
            color_states: &[ColorStateDescriptor {
                format: TextureFormat::Bgra8Unorm,
                color_blend: BlendDescriptor {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha_blend: BlendDescriptor {
                    src_factor: BlendFactor::OneMinusDstAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                write_mask: ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[VertexBufferDescriptor {
                    stride: std::mem::size_of::<Vertex>() as BufferAddress,
                    step_mode: InputStepMode::Vertex,
                    attributes: &vertex_attr_array![0 => Float2, 1 => Float2, 2 => Float3, 3 => Float2, 4 => Float2],
                }],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let image_pipe = device.create_render_pipeline(&pipe_desc);

        pipe_desc.fragment_stage = Some(wgpu::ProgrammableStageDescriptor {
            module: &frag_font_shader,
            entry_point: "main",
        });

        let font_pipe = device.create_render_pipeline(&pipe_desc);

        WgpuRenderer {
            view_matrix_buffer,
            view_matrix_bind_group,
            texture_layout,
            textures: Vec::new(),
            fonts: Vec::new(),
            image_pipe,
            font_pipe,
            device,
            queue,
            draw_list: WgpuDrawList::new(),
            groups: Vec::new(),
        }
    }

    /// Draws the current [`Frame`](struct.Frame.html) to the screen
    pub fn draw_frame<'a>(&'a mut self, frame: Frame, render_pass: &mut RenderPass<'a>) {
        let mouse_cursor = frame.mouse_cursor();
        let (context, widgets, render_groups) = frame.finish_frame();
        let context = context.internal().borrow();

        let time_millis = context.time_millis();
        let scale = context.scale_factor();
        
        self.update_view_matrix(Point::default(), context.display_size());
        self.groups.clear();

        // render all widget groups to buffers
        for render_group in render_groups.into_iter().rev() {
            let mut draw_mode = None;
            self.draw_list.clear();

            // render backgrounds
            for widget in render_group.iter(&widgets) {
                if !widget.visible() { continue; }
                let image_handle = match widget.background() {
                    None => continue,
                    Some(handle) => handle,
                };
                let time_millis = time_millis - context.base_time_millis_for(widget.id());
                let image = context.themes().image(image_handle);
    
                self.buffer_if_changed(&mut draw_mode, DrawMode::Image(image.texture()));

                image.draw(
                    &mut self.draw_list,
                    ImageDrawParams {
                        pos: widget.pos().into(),
                        size: widget.size().into(),
                        anim_state: widget.anim_state(),
                        clip: widget.clip(),
                        time_millis,
                        scale,
                    }
                );
            }

            // render foregrounds & text
            for widget in render_group.iter(&widgets) {
                if !widget.visible() { continue; }

                let border = widget.border();
                let fg_pos = widget.pos() + border.tl();
                let fg_size = widget.inner_size();
    
                if let Some(image_handle) = widget.foreground() {
                    let time_millis = time_millis - context.base_time_millis_for(widget.id());
                    let image = context.themes().image(image_handle);

                    self.buffer_if_changed(&mut draw_mode, DrawMode::Image(image.texture()));

                    image.draw(
                        &mut self.draw_list,
                        ImageDrawParams {
                            pos: fg_pos.into(),
                            size: fg_size.into(),
                            anim_state: widget.anim_state(),
                            clip: widget.clip(),
                            time_millis,
                            scale,
                        }
                    );
                }

                if let Some(text) = widget.text() {
                    if let Some(font_sum) = widget.font() {
                        self.buffer_if_changed(&mut draw_mode, DrawMode::Font(font_sum.handle));
                        let font = context.themes().font(font_sum.handle);
    
                        font.draw(
                            &mut self.draw_list,
                            fg_size * scale,
                            (fg_pos * scale).into(),
                            text,
                            widget.text_align(),
                            widget.text_color(),
                            widget.clip() * scale,
                        )
                    }
                }
            }

            // draw any not already drawn vertices
            if let Some(cur_mode) = draw_mode.take() {
                self.buffer(cur_mode);
            }
        }

        if let Some((mouse_cursor, align, anim_state)) = mouse_cursor {
            let image = context.themes().image(mouse_cursor);
            let mouse_pos = context.mouse_pos();
            let size = image.base_size();
            let pos = mouse_pos - align.adjust_for(size);
            let clip = Rect::new(pos, size);

            let params = ImageDrawParams {
                pos: pos.into(),
                size: size.into(),
                anim_state,
                clip,
                time_millis,
                scale
            };

            self.draw_list.clear();
            image.draw(&mut self.draw_list, params);
            self.buffer(DrawMode::Image(image.texture()));
        }

        // setup view matrix uniform
        render_pass.set_bind_group(0, &self.view_matrix_bind_group, &[]);

        // draw buffers to render pass
        for group in &self.groups {
            let texture = match &group.mode {
                DrawMode::Image(handle) => {
                   render_pass.set_pipeline(&self.image_pipe);
                   &self.textures[handle.id()]
                },
                DrawMode::Font(handle) => {
                    render_pass.set_pipeline(&self.font_pipe);
                    &self.fonts[handle.id()]
                }
            };

            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.set_vertex_buffer(0, group.vertex.slice(..));
            render_pass.set_index_buffer(group.index.slice(..));
            render_pass.draw_indexed(0..group.vertices, 0, 0..1);
        }
    }

    fn buffer_if_changed(
        &mut self,
        mode: &mut Option<DrawMode>,
        desired_mode: DrawMode,
    ) {
        match mode {
            None => *mode = Some(desired_mode),
            Some(cur_mode) => if *cur_mode != desired_mode {
                self.buffer(*cur_mode);
                *mode = Some(desired_mode);
            }
        }
    }

    fn buffer(&mut self, mode: DrawMode) {
        let vertex = self.create_vertex_buffer(&self.draw_list.vertices);
        let index = self.create_index_buffer(&self.draw_list.indices);
        self.groups.push(BufferedGroup {
            vertex,
            index,
            mode,
            vertices: self.draw_list.indices.len() as u32,
        });
        self.draw_list.clear();
    }

    fn create_vertex_buffer(&self, vertices: &[Vertex]) -> Buffer {
        let data = bytemuck::cast_slice(&vertices);
        self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: data,
            usage: BufferUsage::VERTEX,
        })
    }

    fn create_index_buffer(&self, indices: &[u16]) -> Buffer {
        let data = bytemuck::cast_slice(&indices);
        self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("index buffer"),
            contents: data,
            usage: BufferUsage::INDEX,
        })
    }

    fn update_view_matrix(&self, display_pos: Point, display_size: Point) {
        let view_matrix = view_matrix(display_pos, display_size);
        let data = bytemuck::bytes_of(&view_matrix);
        self.queue.write_buffer(&self.view_matrix_buffer, 0, data);
    }

    fn create_texture(&self, image_data: &[u8], width: u32, height: u32, format: wgpu::TextureFormat) -> BindGroup {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d { width, height, depth: 1, },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label: None,
        });

        let bytes = image_data.len();
        self.queue.write_texture(
            TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            },
            image_data,
            TextureDataLayout {
                offset: 0,
                bytes_per_row: bytes as u32 / height,
                rows_per_image: height,
            },
            wgpu::Extent3d { width, height, depth: 1, },
        );

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: None,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: None,
        });

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &self.texture_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        bind_group
    }
}

impl<'a> Renderer for WgpuRenderer {
    fn register_font(
        &mut self,
        handle: crate::render::FontHandle,
        source: &crate::font::FontSource,
        size: f32,
        scale: f32,
    ) -> Result<crate::font::Font, crate::Error> {
        let font = &source.font;

        let writer = FontTextureWriter::new(font, size, scale);
        let writer_out = writer.write(handle)?;

        let bind_group = self.create_texture(
            &writer_out.data,
            writer_out.tex_width,
            writer_out.tex_height,
            wgpu::TextureFormat::R8Unorm,
        );

        assert!(handle.id() == self.fonts.len());
        self.fonts.push(Texture {
            bind_group,
        });

        Ok(writer_out.font)
    }

    fn register_texture(
        &mut self,
        handle: crate::render::TextureHandle,
        image_data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<crate::render::TextureData, crate::Error> {
        let bind_group = self.create_texture(
            image_data,
            dimensions.0,
            dimensions.1,
            wgpu::TextureFormat::Rgba8Unorm
        );

        assert!(handle.id() == self.textures.len());
        self.textures.push(Texture {
            bind_group,
        });

        Ok(TextureData::new(handle, dimensions.0, dimensions.1))
    }
}

struct BufferedGroup {
    vertex: Buffer,
    index: Buffer,
    mode: DrawMode,
    vertices: u32,
}

struct Texture {
    bind_group: BindGroup,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Vertex {
    position: [f32; 2],
    tex: [f32; 2],
    color: [f32; 3],
    clip_pos: [f32; 2],
    clip_size: [f32; 2],
}

// safety - Vertex is exactly 44 bytes with no padding.  all bit patterns are allowed.
unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

struct WgpuDrawList {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl WgpuDrawList {
    fn new() -> Self {
        WgpuDrawList {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

impl DrawList for WgpuDrawList {
    fn len(&self) -> usize { self.vertices.len() }

    fn back_adjust_positions(&mut self, since_index: usize, amount: Point) {
        for vert in self.vertices.iter_mut().skip(since_index) {
            vert.position[0] += amount.x;
            vert.position[1] += amount.y;
        }
    }

    fn push_rect(
        &mut self,
        pos: [f32; 2],
        size: [f32; 2],
        tex: [TexCoord; 2],
        color: Color,
        clip: Rect,
    ) {
        let ul = Vertex {
            position: [pos[0], pos[1]],
            tex: tex[0].into(),
            color: color.into(),
            clip_pos: clip.pos.into(),
            clip_size: clip.size.into(),
        };

        let lr = Vertex {
            position: [pos[0] + size[0], pos[1] + size[1]],
            tex: tex[1].into(),
            color: color.into(),
            clip_pos: clip.pos.into(),
            clip_size: clip.size.into(),
        };

        let idx = self.vertices.len() as u16;
        self.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

        self.vertices.push(ul);
        self.vertices.push(Vertex {
            position: [ul.position[0], lr.position[1]],
            tex: [ul.tex[0], lr.tex[1]],
            color: ul.color,
            clip_pos: clip.pos.into(),
            clip_size: clip.size.into(),
        });
        self.vertices.push(lr);
        self.vertices.push(Vertex {
            position: [lr.position[0], ul.position[1]],
            tex: [lr.tex[0], ul.tex[1]],
            color: lr.color,
            clip_pos: clip.pos.into(),
            clip_size: clip.size.into(),
        });
    }
}