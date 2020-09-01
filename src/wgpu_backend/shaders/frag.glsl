#version 450

layout(set = 1, binding = 0) uniform texture2D tex;
layout(set = 1, binding = 1) uniform sampler samp;

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec3 v_color;

layout(location = 0) out vec4 color;

void main() {
  color = vec4(v_color, 1.0) * texture(sampler2D(tex, samp), v_tex_coords);
}