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
  #version 140

  in vec2 position;
  in vec2 tex;
  in vec3 color;

  layout(location = 0) out vec2 v_tex_coords;
  layout(location = 1) out vec3 v_color;

  void main() {
    gl_Position = vec4(position, 0.0, 1.0);
	
    v_tex_coords = tex;
    v_color = color;
  }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
  #version 140

  layout(location = 0) in vec2 v_tex_coords;
  layout(location = 1) in vec3 v_color;

  out vec4 color;

  uniform sampler2D tex;

  void main() {
    color = vec4(v_color, 1.0) * texture(tex, v_tex_coords);
  }
"#;