use std::collections::HashMap;

use crate::{Error};
use crate::{TexCoord, TextureHandle, TextureData, Color, AnimState, DrawList, Vertex};
use crate::theme_definition::{ImageDefinition, ImageDefinitionKind};

#[derive(Clone)]
enum ImageKind {
    Composed {
        tex_coords: [[TexCoord; 4]; 4],
        grid_size: [f32; 2],
    },
    Simple {
        tex_coords: [TexCoord; 2],
        fixed_size: Option<[f32; 2]>,
    },
    Animated {
        states: Vec<(AnimState, Image)>
    }
}

#[derive(Clone)]
pub struct Image {
    texture: TextureHandle,
    color: Color,
    kind: ImageKind
}

impl Image {
    pub fn texture(&self) -> TextureHandle { self.texture }

    pub fn draw(&self, draw_list: &mut DrawList, pos: [f32; 2], size: [f32; 2], anim_state: AnimState) {
        match &self.kind {
            ImageKind::Composed { tex_coords, grid_size } => {
                self.draw_composed(draw_list, tex_coords, *grid_size, pos, size);
            },
            ImageKind::Simple { tex_coords, fixed_size } => {
                if let Some(size) = fixed_size {
                    self.draw_simple(draw_list, tex_coords, pos, *size);
                } else {
                    self.draw_simple(draw_list, tex_coords, pos, size);
                }
            },
            ImageKind::Animated { states } => {
                self.draw_animated(draw_list, pos, size, anim_state, states);
            }
        }
    }

    pub(crate) fn new(
        image_id: &str,
        def: ImageDefinition,
        texture: &TextureData,
        others: &HashMap<String, Image>
    )-> Result<Image, Error> {
        let kind = match def.kind {
            ImageDefinitionKind::Composed { grid_size, position} => {
                let mut tex_coords = [[TexCoord::default(); 4]; 4];
                for y in 0..4 {
                    #[allow(clippy::needless_range_loop)]
                    for x in 0..4 {
                        let x_val = position[0] + x as u32 * grid_size[0];
                        let y_val = position[1] + y as u32 * grid_size[1];
                        tex_coords[x][y] = texture.tex_coord(x_val, y_val);
                    }
                }

                let grid_size = [grid_size[0] as f32, grid_size[1] as f32];
                ImageKind::Composed { tex_coords, grid_size }
            },
            ImageDefinitionKind::Simple { size, position, stretch } => {
                let tex1 = texture.tex_coord(position[0], position[1]);
                let tex2 = texture.tex_coord(position[0] + size[0], position[1] + size[1]);
                let fixed_size = if !stretch { Some([size[0] as f32, size[1] as f32]) } else { None };
                ImageKind::Simple { tex_coords: [tex1, tex2], fixed_size }
            },
            ImageDefinitionKind::Animated { states } => {
                let mut states_out: Vec<(AnimState, Image)> = Vec::new();
                for (state, id) in states {
                    let image = find_image_in_set(image_id, others, &id)?;
                    states_out.push((state, image));
                }

                ImageKind::Animated { states: states_out }
            }
        };

        Ok(Image {
            color: def.color,
            texture: texture.handle,
            kind
        })
    }

    fn draw_animated(
        &self,
        draw_list: &mut DrawList,
        pos: [f32; 2],
        size: [f32; 2],
        to_find: AnimState,
        states: &[(AnimState, Image)],
    ) {
        for (state, image) in states {
            if state == &to_find {
                image.draw(draw_list, pos, size, to_find);
                break;
            }
        }
    }

    fn draw_simple(
        &self,
        draw_list: &mut DrawList,
        tex: &[TexCoord; 2],
        pos: [f32; 2],
        size: [f32; 2],
    ) {
        draw_list.push_quad(
            Vertex {
                position: [pos[0], pos[1]],
                tex_coords: tex[0].into(),
                color: self.color.into(),
            },
            Vertex {
                position: [pos[0] + size[0], pos[1] + size[1]],
                tex_coords: tex[1].into(),
                color: self.color.into(),
            }
        );
    }

    fn draw_composed(
        &self,
        draw_list: &mut DrawList,
        tex: &[[TexCoord; 4]; 4],
        grid_size: [f32; 2],
        pos: [f32; 2],
        size: [f32; 2]
    ) {
        draw_list.push_quad(
            Vertex {
                position: pos,
                tex_coords: tex[0][0].into(),
                color: self.color.into(),
            },
            Vertex {
                position: [pos[0] + grid_size[0], pos[1] + grid_size[1]],
                tex_coords: tex[1][1].into(),
                color: self.color.into(),
            },
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_quad(
                Vertex {
                    position: [pos[0] + grid_size[0], pos[1]],
                    tex_coords: tex[1][0].into(),
                    color: self.color.into(),
                },
                Vertex {
                    position: [pos[0] + size[0] - grid_size[0], pos[1] + grid_size[1]],
                    tex_coords: tex[2][1].into(),
                    color: self.color.into(),
                }
            );
        }

        draw_list.push_quad(
            Vertex {
                position: [pos[0] + size[0] - grid_size[0], pos[1]],
                tex_coords: tex[2][0].into(),
                color: self.color.into(),
            },
            Vertex {
                position: [pos[0] + size[0], pos[1] + grid_size[1]],
                tex_coords: tex[3][1].into(),
                color: self.color.into(),
            },
        );

        if size[1] > 2.0 * grid_size[1] {
            draw_list.push_quad(
                Vertex {
                    position: [pos[0], pos[1] + grid_size[1]],
                    tex_coords: tex[0][1].into(),
                    color: self.color.into(),
                },
                Vertex {
                    position: [pos[0] + grid_size[0], pos[1] + size[1] - grid_size[1]],
                    tex_coords: tex[1][2].into(),
                    color: self.color.into(),
                },
            );

            if size[0] > 2.0 * grid_size[0] {
                draw_list.push_quad(
                    Vertex {
                        position: [pos[0] + grid_size[0], pos[1] + grid_size[1]],
                        tex_coords: tex[1][1].into(),
                        color: self.color.into(),
                    },
                    Vertex {
                        position: [pos[0] + size[0] - grid_size[0], pos[1] + size[1] - grid_size[1]],
                        tex_coords: tex[2][2].into(),
                        color: self.color.into(),
                    },
                );
            }

            draw_list.push_quad(
                Vertex {
                    position: [pos[0] + size[0] - grid_size[0], pos[1] + grid_size[1]],
                    tex_coords: tex[2][1].into(),
                    color: self.color.into(),
                },
                Vertex {
                    position: [pos[0] + size[0], pos[1] + size[1] - grid_size[1]],
                    tex_coords: tex[3][2].into(),
                    color: self.color.into(),
                }
            );
        }

        draw_list.push_quad(
            Vertex {
                position: [pos[0], pos[1] + size[1] - grid_size[1]],
                tex_coords: tex[0][2].into(),
                color: self.color.into(),
            },
            Vertex {
                position: [pos[0] + grid_size[0], pos[1] + size[1]],
                tex_coords: tex[1][3].into(),
                color: self.color.into(),
            }
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_quad(
                Vertex {
                    position: [pos[0] + grid_size[0], pos[1] + size[1] - grid_size[1]],
                    tex_coords: tex[1][2].into(),
                    color: self.color.into(),
                },
                Vertex {
                    position: [pos[0] + size[0] - grid_size[0], pos[1] + size[1]],
                    tex_coords: tex[2][3].into(),
                    color: self.color.into(),
                }
            );
        }

        draw_list.push_quad(
            Vertex {
                position: [pos[0] + size[0] - grid_size[0], pos[1] + size[1] - grid_size[1]],
                tex_coords: tex[2][2].into(),
                color: self.color.into(),
            },
            Vertex {
                position: [pos[0] + size[0], pos[1] + size[1]],
                tex_coords: tex[3][3].into(),
                color: self.color.into(),
            }
        );
    }
}

fn find_image_in_set(parent_id: &str, set: &HashMap<String, Image>, id: &str) -> Result<Image, Error> {
    match set.get(id) {
        None => {
            Err(
                Error::Theme(format!("Unable to find image '{}' referenced as sub image of '{}'", id, parent_id))
            )
        }, Some(image) => Ok(image.clone())
    }
}