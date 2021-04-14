use crate::font::{Font, FontSource, FontTextureWriter, FontDrawParams};
use crate::image::ImageDrawParams;
use crate::render::{
    view_matrix, DrawList, DrawMode, FontHandle, Renderer, TexCoord, TextureData, TextureHandle,
};
use crate::theme_definition::CharacterRange;
use crate::{Color, Frame, Point, Rect};

mod program;
use program::Program;

mod texture;
use texture::GLTexture;

mod vertex_buffer;
use vertex_buffer::VAO;

/// A Thyme [`Renderer`](trait.Renderer.html) for raw [`OpenGL`](https://github.com/brendanzab/gl-rs/).
///
/// This adapter registers image and font data as OpenGL textures using gl-rs, and renders each frame.
/// After the UI has been built, the [`Frame`](struct.Frame.html) should be passed to the renderer
/// for drawing.
///
/// Fonts are prerendered to a texture on the GPU, based on the ttf
/// font data and the theme specified size.
///
/// Data is structured to minimize number of draw calls, with one to three draw calls per render group
/// (created with [`WidgetBuilder.new_render_group`](struct.WidgetBuilder.html#method.new_render_group))
/// being typical.  Unless you need UI groups where different widgets may overlap and change draw
/// ordering frame-by-frame, a single render group will usually be enough for most of your UI.
///
/// Widget clipping is handled using `gl::CLIP_DISTANCE0` to `gl::CLIP_DISTANCE3`, again to minimize draw calls.  Since the data to send
/// to the GPU is constructed each frame in the immediate mode UI model, the amount of data is minimized
/// by sending only a single `Vertex` for each Image, with the vertex components including the rectangular position and
/// texture coordinates.  The actual individual on-screen vertices are then constructed with a Geometry shader.
pub struct GLRenderer {
    base_program: Program,
    font_program: Program,

    // assets loaded from the context
    textures: Vec<GLTexture>,
    fonts: Vec<GLTexture>,

    // per frame data
    draw_list: GLDrawList,
    groups: Vec<DrawGroup>,
    matrix: [[f32; 4]; 4],
}

impl Default for GLRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl GLRenderer {
    /// Creates a GLRenderer
    pub fn new() -> GLRenderer {
        let base_program = Program::new(VERT_SHADER_SRC, GEOM_SHADER_SRC, FRAGMENT_SHADER_SRC);

        let font_program = Program::new(VERT_SHADER_SRC, GEOM_SHADER_SRC, FONT_FRAGMENT_SHADER_SRC);

        GLRenderer {
            base_program,
            font_program,
            fonts: Vec::new(),
            textures: Vec::new(),
            draw_list: GLDrawList::new(),
            groups: Vec::new(),
            matrix: view_matrix(Point::default(), Point { x: 100.0, y: 100.0 }),
        }
    }

    fn font(&self, font: FontHandle) -> &GLTexture {
        &self.fonts[font.id()]
    }

    fn texture(&self, texture: TextureHandle) -> &GLTexture {
        &self.textures[texture.id()]
    }

    /// Clears the screen with this color.
    pub fn clear_color(&self, r: f32, g: f32, b: f32, a: f32) {
        unsafe {
            gl::ClearColor(r, g, b, a);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    /// Draws the specified [`Frame`](struct.Frame.html) to the Glium surface, usually the Glium Frame.
    pub fn draw_frame(&mut self, frame: Frame) {
        let mouse_cursor = frame.mouse_cursor();
        let (context, widgets, render_groups) = frame.finish_frame();
        let context = context.internal().borrow();

        let time_millis = context.time_millis();
        let display_pos = Point::default();
        let display_size = context.display_size();
        let scale = context.scale_factor();
        self.matrix = view_matrix(display_pos, display_size);

        self.draw_list.clear();
        self.groups.clear();

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::CLIP_DISTANCE0);
            gl::Enable(gl::CLIP_DISTANCE1);
            gl::Enable(gl::CLIP_DISTANCE2);
            gl::Enable(gl::CLIP_DISTANCE3);
        }

        for render_group in render_groups.into_iter().rev() {
            let mut draw_mode = None;

            // render backgrounds
            for widget in render_group.iter(&widgets) {
                if !widget.visible() {
                    continue;
                }
                let image_handle = match widget.background() {
                    None => continue,
                    Some(handle) => handle,
                };
                let time_millis = time_millis - context.base_time_millis_for(widget.id());
                let image = context.themes().image(image_handle);

                self.write_group_if_changed(&mut draw_mode, DrawMode::Image(image.texture()));

                image.draw(
                    &mut self.draw_list,
                    ImageDrawParams {
                        pos: widget.pos().into(),
                        size: widget.size().into(),
                        anim_state: widget.anim_state(),
                        clip: widget.clip(),
                        time_millis,
                        scale,
                    },
                );
            }

            // render foregrounds & text
            for widget in render_group.iter(&widgets) {
                if !widget.visible() {
                    continue;
                }

                let border = widget.border();
                let fg_pos = widget.pos() + border.tl();
                let fg_size = widget.inner_size();

                if let Some(image_handle) = widget.foreground() {
                    let time_millis = time_millis - context.base_time_millis_for(widget.id());
                    let image = context.themes().image(image_handle);
                    self.write_group_if_changed(&mut draw_mode, DrawMode::Image(image.texture()));

                    image.draw(
                        &mut self.draw_list,
                        ImageDrawParams {
                            pos: fg_pos.into(),
                            size: fg_size.into(),
                            anim_state: widget.anim_state(),
                            clip: widget.clip(),
                            time_millis,
                            scale,
                        },
                    );
                }

                if let Some(text) = widget.text() {
                    if let Some(font_sum) = widget.font() {
                        self.write_group_if_changed(
                            &mut draw_mode,
                            DrawMode::Font(font_sum.handle),
                        );
                        let font = context.themes().font(font_sum.handle);

                        let params = FontDrawParams {
                            area_size: fg_size * scale,
                            pos: fg_pos * scale,
                            indent: widget.text_indent(),
                            align: widget.text_align(),
                        };

                        font.draw(
                            &mut self.draw_list,
                            params,
                            text,
                            widget.text_color(),
                            widget.clip() * scale,
                        )
                    }
                }
            }

            // render anything from the final draw calls
            if let Some(mode) = draw_mode {
                self.write_group(mode);
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
                scale,
            };

            image.draw(&mut self.draw_list, params);
            self.write_group(DrawMode::Image(image.texture()));
        }

        unsafe {
            gl::Enable(gl::FRAMEBUFFER_SRGB);
        }
        // create the vertex buffer and draw all groups
        let vao = VAO::new(&self.draw_list.vertices);
        vao.bind();

        let font_uniform_tex = self.font_program.get_uniform_location("tex");
        let font_uniform_matrix = self.font_program.get_uniform_location("matrix");

        let base_uniform_tex = self.base_program.get_uniform_location("tex");
        let base_uniform_matrix = self.base_program.get_uniform_location("matrix");

        for group in &self.groups {
            match group.mode {
                DrawMode::Font(font_handle) => {
                    let font = self.font(font_handle);

                    font.bind(0);
                    self.font_program.use_program();

                    self.font_program
                        .uniform_matrix4fv(font_uniform_matrix, false, &self.matrix);
                    self.font_program.uniform1i(font_uniform_tex, 0);

                    unsafe {
                        gl::DrawArrays(gl::POINTS, group.start as _, (group.end - group.start) as _)
                    };
                }
                DrawMode::Image(tex_handle) => {
                    let texture = self.texture(tex_handle);

                    texture.bind(0);
                    self.base_program.use_program();

                    self.base_program.uniform1i(base_uniform_tex, 0);
                    self.base_program
                        .uniform_matrix4fv(base_uniform_matrix, false, &self.matrix);

                    unsafe {
                        gl::Disable(gl::FRAMEBUFFER_SRGB);
                    }
                    unsafe {
                        gl::DrawArrays(gl::POINTS, group.start as _, (group.end - group.start) as _)
                    };
                }
            };
        }
    }

    fn write_group_if_changed(&mut self, mode: &mut Option<DrawMode>, desired_mode: DrawMode) {
        match mode {
            None => *mode = Some(desired_mode),
            Some(cur_mode) => {
                if *cur_mode != desired_mode {
                    self.write_group(*cur_mode);
                    *mode = Some(desired_mode);
                }
            }
        }
    }

    fn write_group(&mut self, mode: DrawMode) {
        let end = self.draw_list.vertices.len();
        // if this is the first draw group, start at 0
        let start = match self.groups.last() {
            None => 0,
            Some(group) => group.end,
        };
        self.groups.push(DrawGroup { start, end, mode });
    }
}

impl Renderer for GLRenderer {
    fn register_texture(
        &mut self,
        handle: TextureHandle,
        image_data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<TextureData, crate::Error> {
        let gl_texture = GLTexture::new(
            image_data,
            dimensions,
            gl::LINEAR,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            gl::RGBA8,
        );

        assert!(handle.id() <= self.textures.len());
        if handle.id() == self.textures.len() {
            self.textures.push(gl_texture);
        } else {
            self.textures[handle.id()] = gl_texture;
        }

        Ok(TextureData::new(handle, dimensions.0, dimensions.1))
    }

    fn register_font(
        &mut self,
        handle: FontHandle,
        source: &FontSource,
        ranges: &[CharacterRange],
        size: f32,
        scale: f32,
    ) -> Result<Font, crate::Error> {
        let font = &source.font;

        let writer = FontTextureWriter::new(font, ranges, size, scale);

        let writer_out = writer.write(handle, ranges)?;

        let font_texture = GLTexture::new(
            &writer_out.data,
            (writer_out.tex_width, writer_out.tex_height),
            gl::NEAREST,
            gl::CLAMP_TO_BORDER,
            gl::RED,
            gl::R8,
        );

        assert!(handle.id() <= self.fonts.len());
        if handle.id() == self.fonts.len() {
            self.fonts.push(font_texture);
        } else {
            self.fonts[handle.id()] = font_texture;
        }

        Ok(writer_out.font)
    }
}

struct DrawGroup {
    start: usize,
    end: usize,
    mode: DrawMode,
}

// Pass through the vertex to the geometry shader where the rectangle is built
const VERT_SHADER_SRC: &str = r#"
  #version 330

  layout(location = 0) in vec2 position;
  layout(location = 1) in vec2 size;
  layout(location = 2) in vec2 tex0;
  layout(location = 3) in vec2 tex1;
  layout(location = 4) in vec4 color;
  layout(location = 5) in vec2 clip_pos;
  layout(location = 6) in vec2 clip_size;

  out vec2 g_size;
  out vec2 g_tex0;
  out vec2 g_tex1;
  out vec4 g_color;
  out vec2 g_clip_pos;
  out vec2 g_clip_size;

  void main() {
    gl_Position = vec4(position, 0.0, 1.0);
	
	g_size = size;
	g_tex0 = tex0;
	g_tex1 = tex1;
	g_color = color;
	g_clip_pos = clip_pos;
	g_clip_size = clip_size;
  }
"#;

const GEOM_SHADER_SRC: &str = r#"
  #version 150

  layout (points) in;
  layout (triangle_strip, max_vertices = 4) out;

  in vec2 g_size[];
  in vec2 g_tex0[];
  in vec2 g_tex1[];
  in vec4 g_color[];
  in vec2 g_clip_pos[];
  in vec2 g_clip_size[];

  out vec2 v_tex_coords;
  out vec4 v_color;

  uniform mat4 matrix;

  void main() {
	vec4 base = gl_in[0].gl_Position;
    
    vec2 clip_pos = g_clip_pos[0];
    vec2 clip_size = g_clip_size[0];

    // draw the rectangle using 2 triangles in triangle_strip

    // [0, 0] vertex
    vec4 position = base;
    gl_ClipDistance[0] = position.x - clip_pos.x;
    gl_ClipDistance[1] = clip_pos.x + clip_size.x - position.x;
    gl_ClipDistance[2] = position.y - clip_pos.y;
    gl_ClipDistance[3] = clip_pos.y + clip_size.y - position.y;
	gl_Position = matrix * position;
	v_tex_coords = g_tex0[0];
	v_color = g_color[0];
	EmitVertex();
    
    // [0, 1] vertex
    position = base + vec4(0.0, g_size[0].y, 0.0, 0.0);
    gl_ClipDistance[0] = position.x - clip_pos.x;
    gl_ClipDistance[1] = clip_pos.x + clip_size.x - position.x;
    gl_ClipDistance[2] = position.y - clip_pos.y;
    gl_ClipDistance[3] = clip_pos.y + clip_size.y - position.y;
	gl_Position = matrix * position;
	v_tex_coords = vec2(g_tex0[0].x, g_tex1[0].y);
	v_color = g_color[0];
    EmitVertex();
    
    // [1, 0] vertex
    position = base + vec4(g_size[0].x, 0.0, 0.0, 0.0);
	gl_ClipDistance[0] = position.x - clip_pos.x;
    gl_ClipDistance[1] = clip_pos.x + clip_size.x - position.x;
    gl_ClipDistance[2] = position.y - clip_pos.y;
    gl_ClipDistance[3] = clip_pos.y + clip_size.y - position.y;
	gl_Position = matrix * position;
	v_tex_coords = vec2(g_tex1[0].x, g_tex0[0].y);
	v_color = g_color[0];
    EmitVertex();
    
    // [1, 1] vertex
    position = base + vec4(g_size[0].x, g_size[0].y, 0.0, 0.0);
    gl_ClipDistance[0] = position.x - clip_pos.x;
    gl_ClipDistance[1] = clip_pos.x + clip_size.x - position.x;
    gl_ClipDistance[2] = position.y - clip_pos.y;
    gl_ClipDistance[3] = clip_pos.y + clip_size.y - position.y;
    gl_Position = matrix * position;
    v_tex_coords = g_tex1[0];
    v_color = g_color[0];
    EmitVertex();

    EndPrimitive();
  }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
  #version 150

  in vec2 v_tex_coords;
  in vec4 v_color;

  out vec4 color;

  uniform sampler2D tex;

  void main() {
    color = v_color * texture(tex, v_tex_coords);
  }
"#;

const FONT_FRAGMENT_SHADER_SRC: &str = r#"
    #version 150

    in vec2 v_tex_coords;
    in vec4 v_color;

    out vec4 color;

    uniform sampler2D tex;
    
    void main() {
        color = vec4(v_color.rgb, texture(tex, v_tex_coords).r);
    }
"#;

struct GLDrawList {
    vertices: Vec<GLVertex>,
}

impl GLDrawList {
    fn new() -> Self {
        GLDrawList {
            vertices: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
    }
}

impl DrawList for GLDrawList {
    fn len(&self) -> usize {
        self.vertices.len()
    }

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
        let vert = GLVertex {
            position: pos,
            size,
            tex0: [tex[0].x(), tex[0].y()],
            tex1: [tex[1].x(), tex[1].y()],
            color: color.into(),
            clip_pos: clip.pos.into(),
            clip_size: clip.size.into(),
        };

        self.vertices.push(vert);
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) struct GLVertex {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub tex0: [f32; 2],
    pub tex1: [f32; 2],
    pub color: [f32; 4],
    pub clip_pos: [f32; 2],
    pub clip_size: [f32; 2],
}
