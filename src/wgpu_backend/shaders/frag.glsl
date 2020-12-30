#version 450

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler samp;

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec4 v_color;
layout(location = 2) in vec4 v_clip;

layout(location = 0) out vec4 color;

void main() {
  if (v_clip.x < 0.0 || v_clip.y < 0.0 || v_clip.z < 0.0 || v_clip.w < 0.0) {
    discard;
  }

  color = v_color * texture(sampler2D(tex, samp), v_tex_coords);
}