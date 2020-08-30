use crate::Renderer;

pub struct WgpuRenderer {

}

impl WgpuRenderer {
    pub fn new() -> WgpuRenderer {
        build_spirv();

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

fn build_spirv() {
    use shaderc;

    let mut compiler = shaderc::Compiler::new().unwrap();
    let vert_result = compiler.compile_into_spirv(
        VERT_SHADER_SRC,
        shaderc::ShaderKind::Vertex,
        "vert.glsl",
        "main",
        None
    ).unwrap();

    let frag_result = compiler.compile_into_spirv(
        FRAGMENT_SHADER_SRC,
        shaderc::ShaderKind::Fragment,
        "frag.glsl",
        "main",
        None
    ).unwrap();
}

const VERT_SHADER_SRC: &str = r#"
  #version 420

  layout(set = 0, binding = 0) uniform View {
      mat4 matrix;
  };

  layout(location = 0) in vec2 position;
  layout(location = 1) in vec2 tex;
  layout(location = 2) in vec3 color;

  layout(location = 0) out vec2 v_tex_coords;
  layout(location = 1) out vec3 v_color;

  void main() {
    gl_Position = matrix * vec4(position, 0.0, 1.0);
	
    v_tex_coords = tex;
    v_color = color;
  }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
  #version 420

  layout(set = 1, binding = 0) uniform sampler2D tex;

  layout(location = 0) in vec2 v_tex_coords;
  layout(location = 1) in vec3 v_color;

  layout(location = 0) out vec4 color;

  void main() {
    color = vec4(v_color, 1.0) * texture(tex, v_tex_coords);
  }
"#;