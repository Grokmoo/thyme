use std::collections::HashMap;

use crate::{Error};
use crate::render::{TexCoord, DrawList, TextureHandle, TextureData};
use crate::{Rect, Color, AnimState, Point};
use crate::theme_definition::{ImageDefinition, ImageDefinitionKind};

#[derive(Copy, Clone)]
pub struct ImageHandle {
    pub(crate) id: usize,
}

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
    Timed {
        frame_time_millis: u32,
        frames: Vec<Image>,
        once: bool,
    },
    Animated {
        states: Vec<(AnimState, Image)>
    }
}

pub(crate) struct ImageDrawParams {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub anim_state: AnimState,
    pub clip: Rect,
    pub time_millis: u32,
}

#[derive(Clone)]
pub struct Image {
    texture: TextureHandle,
    color: Color,
    kind: ImageKind,
    base_size: Point,
}

impl Image {
    pub fn texture(&self) -> TextureHandle { self.texture }

    pub fn base_size(&self) -> Point { self.base_size }

    pub(crate) fn draw<D: DrawList>(
        &self,
        draw_list: &mut D,
        params: ImageDrawParams,
    ) {
        match &self.kind {
            ImageKind::Composed { tex_coords, grid_size } => {
                self.draw_composed(draw_list, tex_coords, *grid_size, params.pos, params.size, params.clip);
            },
            ImageKind::Simple { tex_coords, fixed_size } => {
                if let Some(size) = fixed_size {
                    self.draw_simple(draw_list, tex_coords, params.pos, *size, params.clip);
                } else {
                    self.draw_simple(draw_list, tex_coords, params.pos, params.size, params.clip);
                }
            },
            ImageKind::Timed { frame_time_millis, frames, once } => {
                let total_time_millis = frame_time_millis * frames.len() as u32;
                if *once && params.time_millis > total_time_millis {
                    frames[frames.len() - 1].draw(draw_list, params);
                } else {
                    let frame_index = ((params.time_millis % total_time_millis) / frame_time_millis) as usize;
                    frames[frame_index].draw(draw_list, params);
                }
            },
            ImageKind::Animated { states } => {
                self.draw_animated(draw_list, states, params);
            }
        }
    }

    pub(crate) fn new(
        image_id: &str,
        def: ImageDefinition,
        texture: &TextureData,
        others: &HashMap<String, Image>
    )-> Result<Image, Error> {
        let base_size;
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
                base_size = Point::new(grid_size[0] * 3.0, grid_size[1] * 3.0);
                ImageKind::Composed { tex_coords, grid_size }
            },
            ImageDefinitionKind::Simple { size, position, stretch } => {
                let tex1 = texture.tex_coord(position[0], position[1]);
                let tex2 = texture.tex_coord(position[0] + size[0], position[1] + size[1]);
                let fixed_size = if !stretch { Some([size[0] as f32, size[1] as f32]) } else { None };
                base_size = Point::new(size[0] as f32, size[1] as f32);
                ImageKind::Simple { tex_coords: [tex1, tex2], fixed_size }
            },
            ImageDefinitionKind::Timed { frame_time_millis, frames, once } => {
                let mut size = Point::default();
                let mut frames_out = Vec::new();
                for id in frames {
                    let image = find_image_in_set(image_id, others, &id)?;
                    size = image.base_size;
                    frames_out.push(image);
                }
                
                if frames_out.is_empty() {
                    return Err(
                        Error::Theme(format!("No frames specified for image: {}", image_id))
                    );
                }

                base_size = size;
                ImageKind::Timed { frame_time_millis, frames: frames_out, once }
            },
            ImageDefinitionKind::Animated { states } => {
                let mut size = Point::default();
                let mut states_out: Vec<(AnimState, Image)> = Vec::new();
                for (state, id) in states {
                    let image = find_image_in_set(image_id, others, &id)?;
                    size = image.base_size;
                    states_out.push((state, image));
                }

                base_size = size;
                ImageKind::Animated { states: states_out }
            }
        };

        Ok(Image {
            color: def.color,
            texture: texture.handle(),
            kind,
            base_size,
        })
    }

    fn draw_animated<D: DrawList>(
        &self,
        draw_list: &mut D,
        states: &[(AnimState, Image)],
        params: ImageDrawParams,
    ) {
        for (state, image) in states {
            if state == &params.anim_state {
                image.draw(draw_list, params);
                break;
            }
        }
    }

    fn draw_simple<D: DrawList>(
        &self,
        draw_list: &mut D,
        tex: &[TexCoord; 2],
        pos: [f32; 2],
        size: [f32; 2],
        clip: Rect,
    ) {
        draw_list.push_rect(
            [pos[0], pos[1]],
            [size[0], size[1]],
            *tex,
            self.color,
            clip
        );
    }

    fn draw_composed<D: DrawList>(
        &self,
        draw_list: &mut D,
        tex: &[[TexCoord; 4]; 4],
        grid_size: [f32; 2],
        pos: [f32; 2],
        size: [f32; 2],
        clip: Rect,
    ) {
        draw_list.push_rect(
            pos,
            grid_size,
            [tex[0][0], tex[1][1]],
            self.color,
            clip,
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_rect(
                [pos[0] + grid_size[0], pos[1]],
                [size[0] - 2.0 * grid_size[0], grid_size[1]],
                [tex[1][0], tex[2][1]],
                self.color,
                clip,
            );
        }

        draw_list.push_rect(
            [pos[0] + size[0] - grid_size[0], pos[1]],
            grid_size,
            [tex[2][0], tex[3][1]],
            self.color,
            clip,
        );

        if size[1] > 2.0 * grid_size[1] {
            draw_list.push_rect(
                [pos[0], pos[1] + grid_size[1]],
                [grid_size[0], size[1] - 2.0 * grid_size[1]],
                [tex[0][1], tex[1][2]],
                self.color,
                clip,
            );

            if size[0] > 2.0 * grid_size[0] {
                draw_list.push_rect(
                    [pos[0] + grid_size[0], pos[1] + grid_size[1]],
                    [size[0] - 2.0 * grid_size[0], size[1] - 2.0 * grid_size[1]],
                    [tex[1][1], tex[2][2]],
                    self.color,
                    clip,
                );
            }

            draw_list.push_rect(
                [pos[0] + size[0] - grid_size[0], pos[1] + grid_size[1]],
                [grid_size[0], size[1] - 2.0 * grid_size[1]],
                [tex[2][1], tex[3][2]],
                self.color,
                clip,
            );
        }

        draw_list.push_rect(
            [pos[0], pos[1] + size[1] - grid_size[1]],
            grid_size,
            [tex[0][2], tex[1][3]],
            self.color,
            clip,
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_rect(
                [pos[0] + grid_size[0], pos[1] + size[1] - grid_size[1]],
                [size[0] - 2.0 * grid_size[0], grid_size[1]],
                [tex[1][2], tex[2][3]],
                self.color,
                clip,
            );
        }

        draw_list.push_rect(
            [pos[0] + size[0] - grid_size[0], pos[1] + size[1] - grid_size[1]],
            grid_size,
            [tex[2][2], tex[3][3]],
            self.color,
            clip,
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