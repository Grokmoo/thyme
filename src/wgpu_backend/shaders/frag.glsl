#version 420

layout(set = 1, binding = 0) uniform sampler2D tex;

layout(location = 0) in vec2 v_tex_coords;
layout(location = 1) in vec3 v_color;

layout(location = 0) out vec4 color;

void main() {
  color = vec4(v_color, 1.0) * texture(tex, v_tex_coords);
}