use std::rc::Rc;
use std::fmt::Display;
use std::error::Error;
use std::borrow::Cow;

use glium::{implement_vertex, uniform, DrawParameters, program::{ProgramCreationError}, Program, Surface};
use glium::backend::{Context, Facade};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerBehavior, SamplerWrapFunction};
use glium::texture::{Texture2d, RawImage2d, SrgbTexture2d};
use glium::index::PrimitiveType;

use crate::{image::ImageDrawParams};
use crate::render::{view_matrix, TexCoord, DrawList, DrawMode, Renderer, TextureHandle, TextureData, FontHandle};
use crate::font::{Font, FontSource, FontTextureWriter};
use crate::{Frame, Point, Color, Rect};

/// A Thyme [`Renderer`](trait.Renderer.html) for [`Glium`](https://github.com/glium/glium).
///
/// This adapter registers image and font data as OpenGL textures using glium, and renders each frame.
/// After the UI has been built, the [`Frame`](struct.Frame.html) should be passed to the renderer
/// for drawing.
///
/// This adapter is written with overall performance in mind and is fairly fast, able to render fairly complex
/// UIs consisting of many widgets and text in less than 1 ms on moderately powerful GPUs.
///
/// Fonts are prerendered to a texture on the GPU, based on the ttf
/// font data and the theme specified size.
///
/// Data is structured to minimize number of draw calls, with one to three draw calls per render group
/// (created with [`WidgetBuilder.new_render_group`](struct.WidgetBuilder.html#method.new_render_group))
/// being typical.  Unless you need UI groups where different widgets may overlap and change draw
/// ordering frame-by-frame, a single render group will usually be enough for most frames
/// of your UI.
///
/// Widget clipping is handled using `glClipDistance`, again to minimize draw calls.  Since the data to send
/// to the GPU is constructed each frame in the immediate mode UI model, the amount of data is minimized
/// by sending only a single `Vertex` for each Image, with the vertex components including the rectangular position and
/// texture coordinates.  The actual individual on-screen vertices are then constructed with a Geometry shader.
pub struct GliumRenderer {
    context: Rc<Context>,
    base_program: Program,
    font_program: Program,

    // assets loaded from the context
    textures: Vec<GliumTexture>,
    fonts: Vec<GliumFont>,

    // per frame data
    draw_list: GliumDrawList,
    matrix: [[f32; 4]; 4],
    params: DrawParameters<'static>,
}

impl GliumRenderer {
    /// Creates a new [`Renderer`](trait.Renderer.html) to draw to the specified Glium facade.
    pub fn new<F: Facade>(facade: &F) -> Result<GliumRenderer, GliumError> {
        let context = Rc::clone(facade.get_context());

        let base_program = Program::from_source(
            facade,
            VERT_SHADER_SRC,
            FRAGMENT_SHADER_SRC,
            Some(GEOM_SHADER_SRC),
        )?;

        let font_program = Program::from_source(
            facade,
            VERT_SHADER_SRC,
            FONT_FRAGMENT_SHADER_SRC,
            Some(GEOM_SHADER_SRC)
        )?;

        Ok(GliumRenderer {
            context,
            base_program,
            font_program,
            fonts: Vec::new(),
            textures: Vec::new(),
            draw_list: GliumDrawList::new(),
            matrix: view_matrix(Point::default(), Point { x: 100.0, y: 100.0 }),
            params: DrawParameters {
                blend: glium::Blend::alpha_blending(),
                clip_planes_bitmask: 0b1111, //enable the first 4 clip planes
                ..DrawParameters::default()
            },
        })
    }

    fn font(&self, font: FontHandle) -> &GliumFont {
        &self.fonts[font.id()]
    }

    fn texture(&self, texture: TextureHandle) -> &GliumTexture {
        &self.textures[texture.id()]
    }

    /// Draws the specified [`Frame`](struct.Frame.html) to the Glium surface, usually the Glium Frame.
    pub fn draw_frame<T: Surface>(&mut self, target: &mut T, frame: Frame) -> Result<(), GliumError> {
        let mouse_cursor = frame.mouse_cursor();
        let (context, widgets, render_groups) = frame.finish_frame();
        let context = context.internal().borrow();

        let time_millis = context.time_millis();
        let display_pos = Point::default();
        let display_size = context.display_size();
        let scale = context.scale_factor();
        self.matrix = view_matrix(display_pos, display_size);

        for render_group in render_groups.into_iter() {
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
    
                self.render_if_changed(target, &mut draw_mode, DrawMode::Image(image.texture()))?;
                
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
                    self.render_if_changed(target, &mut draw_mode, DrawMode::Image(image.texture()))?;
    
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
                        self.render_if_changed(target, &mut draw_mode, DrawMode::Font(font_sum.handle))?;
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

            // render anything from the final draw calls
            if let Some(mode) = draw_mode {
                self.render(target, mode)?;
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
            self.render(target, DrawMode::Image(image.texture()))?;
        }

        Ok(())
    }

    fn render_if_changed<T: Surface>(
        &mut self,
        target: &mut T,
        mode: &mut Option<DrawMode>,
        desired_mode: DrawMode,
    ) -> Result<(), GliumError> {
        match mode {
            None => *mode = Some(desired_mode),
            Some(cur_mode) => if *cur_mode != desired_mode {
                self.render(target, *cur_mode)?;
                *mode = Some(desired_mode);
            }
        }

        Ok(())
    }

    fn render<T: Surface>(
        &mut self,
        target: &mut T,
        mode: DrawMode,
    ) -> Result<(), GliumError> {
        let vertices = glium::VertexBuffer::immutable(
            &self.context, &self.draw_list.vertices
        )?;
        let indices = glium::index::NoIndices(PrimitiveType::Points);
        match mode {
            DrawMode::Font(font_handle) => {
                let font = self.font(font_handle);
                let uniforms = uniform! {
                    tex: Sampler(&font.texture, font.sampler),
                    matrix: self.matrix,
                };
                target.draw(&vertices, &indices, &self.font_program, &uniforms, &self.params)?;
            },
            DrawMode::Image(tex_handle) => {
                let texture = self.texture(tex_handle);
                let uniforms = uniform! {
                    tex: Sampler(&texture.texture, texture.sampler),
                    matrix: self.matrix,
                };
                target.draw(&vertices, &indices, &self.base_program, &uniforms, &self.params)?;
            }
        };

        self.draw_list.clear();
        Ok(())
    }
}

impl Renderer for GliumRenderer {
    fn register_texture(
        &mut self,
        handle: TextureHandle,
        image_data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<TextureData, crate::Error> {
        let image = RawImage2d::from_raw_rgba(image_data.to_vec(), dimensions);
        let texture = SrgbTexture2d::new(&self.context, image).unwrap();

        let sampler = SamplerBehavior {
            minify_filter: MinifySamplerFilter::Linear,
            magnify_filter: MagnifySamplerFilter::Nearest,
            wrap_function: (
                SamplerWrapFunction::Clamp,
                SamplerWrapFunction::Clamp,
                SamplerWrapFunction::Clamp,
            ),
            ..Default::default()
        };

        assert!(handle.id() <= self.textures.len());
        if handle.id() == self.textures.len() {
            self.textures.push(GliumTexture { texture, sampler });
        } else {
            self.textures[handle.id()] = GliumTexture { texture, sampler };
        }

        Ok(TextureData::new(handle, dimensions.0, dimensions.1))
    }

    fn register_font(
        &mut self,
        handle: FontHandle,
        source: &FontSource,
        size: f32,
        scale: f32,
    ) -> Result<Font, crate::Error> {
        let font = &source.font;

        let writer = FontTextureWriter::new(font, size, scale);

        let writer_out = writer.write(handle)?;

        let font_tex = Texture2d::with_format(
            &self.context,
            RawImage2d {
                data: Cow::Owned(writer_out.data),
                width: writer_out.tex_width,
                height: writer_out.tex_height,
                format: glium::texture::ClientFormat::U8,
            },
            glium::texture::UncompressedFloatFormat::U8,
            glium::texture::MipmapsOption::NoMipmap,
        ).unwrap();

        let sampler = SamplerBehavior {
            minify_filter: MinifySamplerFilter::Nearest,
            magnify_filter: MagnifySamplerFilter::Nearest,
            wrap_function: (
                SamplerWrapFunction::BorderClamp,
                SamplerWrapFunction::BorderClamp,
                SamplerWrapFunction::BorderClamp,
            ),
            ..Default::default()
        };

        assert!(handle.id() <= self.fonts.len());
        if handle.id() == self.fonts.len() {
            self.fonts.push(GliumFont { texture: font_tex, sampler });
        } else {
            self.fonts[handle.id()] = GliumFont { texture: font_tex, sampler };
        }
        

        Ok(writer_out.font)
    }
}

struct GliumFont {
    texture: Texture2d,
    sampler: SamplerBehavior,
}

struct GliumTexture {
    texture: SrgbTexture2d,
    sampler: SamplerBehavior,
}

#[derive(Debug)]
pub enum GliumError {
    Draw(glium::DrawError),
    Index(glium::index::BufferCreationError),
    Font(String),
    InvalidTexture(TextureHandle),
    InvalidFont(FontHandle),
    Program(ProgramCreationError),
    Vertex(glium::vertex::BufferCreationError),
}

impl Display for GliumError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::GliumError::*;
        match self {
            Draw(e) => write!(f, "Error drawing to target: {}", e),
            Index(e) => write!(f, "Index buffer creation failed: {}", e),
            Font(e) => write!(f, "{}", e),
            InvalidTexture(handle) => write!(f, "Invalid texture: {:?}", handle),
            InvalidFont(handle) => write!(f, "Invalid font: {:?}", handle),
            Program(e) => write!(f, "Shader program creation failed: {}", e),
            Vertex(e) => write!(f, "Vertex buffer creation failed: {}", e),

        }
    }
}

impl Error for GliumError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::GliumError::*;
        match self {
            Draw(e) => Some(e),
            InvalidTexture(_) => None,
            InvalidFont(_) => None,
            Font(_) => None,
            Index(e) => Some(e),
            Program(e) => Some(e),
            Vertex(e) => Some(e),
        }
    }
}

impl From<glium::index::BufferCreationError> for GliumError {
    fn from(e: glium::index::BufferCreationError) -> GliumError {
        GliumError::Index(e)
    }
}

impl From<glium::vertex::BufferCreationError> for GliumError {
    fn from(e: glium::vertex::BufferCreationError) -> GliumError {
        GliumError::Vertex(e)
    }
}

impl From<glium::DrawError> for GliumError {
    fn from(e: glium::DrawError) -> GliumError {
        GliumError::Draw(e)
    }
}

impl From<ProgramCreationError> for GliumError {
    fn from(e: ProgramCreationError) -> GliumError {
        GliumError::Program(e)
    }
}

// Pass through the vertex to the geometry shader where the rectangle is built
const VERT_SHADER_SRC: &str = r#"
  #version 140

  in vec2 position;
  in vec2 size;
  in vec2 tex0;
  in vec2 tex1;
  in vec3 color;
  in vec2 clip_pos;
  in vec2 clip_size;

  out vec2 g_size;
  out vec2 g_tex0;
  out vec2 g_tex1;
  out vec3 g_color;
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
  #version 150 core

  layout (points) in;
  layout (triangle_strip, max_vertices = 4) out;

  in vec2 g_size[];
  in vec2 g_tex0[];
  in vec2 g_tex1[];
  in vec3 g_color[];
  in vec2 g_clip_pos[];
  in vec2 g_clip_size[];

  out vec2 v_tex_coords;
  out vec3 v_color;

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
  #version 140

  in vec2 v_tex_coords;
  in vec3 v_color;

  out vec4 color;

  uniform sampler2D tex;

  void main() {
    color = vec4(v_color, 1.0) * texture(tex, v_tex_coords);
  }
"#;

const FONT_FRAGMENT_SHADER_SRC: &str = r#"
    #version 140

    in vec2 v_tex_coords;
    in vec3 v_color;

    out vec4 color;

    uniform sampler2D tex;
    
    void main() {
        color = vec4(v_color, texture(tex, v_tex_coords).r);
    }
"#;

struct GliumDrawList {
    vertices: Vec<GliumVertex>,
}

impl GliumDrawList {
    fn new() -> Self {
        GliumDrawList {
            vertices: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
    }
}

impl DrawList for GliumDrawList {
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
        let vert = GliumVertex {
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
struct GliumVertex {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub tex0: [f32; 2],
    pub tex1: [f32; 2],
    pub color: [f32; 3],
    pub clip_pos: [f32; 2],
    pub clip_size: [f32; 2],
}

implement_vertex!(GliumVertex, position, size, tex0, tex1, color, clip_pos, clip_size);
