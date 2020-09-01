#version 450

layout(set = 0, binding = 0) uniform View { mat4 matrix; };

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex;
layout(location = 2) in vec3 color;
layout(location = 3) in vec2 clip_pos;
layout(location = 4) in vec2 clip_size;

layout(location = 0) out vec2 v_tex_coords;
layout(location = 1) out vec3 v_color;
layout(location = 2) out vec4 v_clip;

void main() {
  gl_Position = matrix * vec4(position, 0.0, 1.0);

  v_tex_coords = tex;
  v_color = color;
  
  v_clip = vec4(
    position.x - clip_pos.x,
    clip_pos.x + clip_size.x - position.x,
    position.y - clip_pos.y,
    clip_pos.y + clip_size.y - position.y
  );
}