use std::collections::HashMap;

use crate::{Error};
use crate::render::{TexCoord, DrawList, TextureHandle, TextureData};
use crate::{Rect, Color, AnimState, Point};
use crate::theme_definition::{ImageFill, ImageDefinition, ImageDefinitionKind};

#[derive(Copy, Clone)]
pub struct ImageHandle {
    pub(crate) id: usize,
}

#[derive(Clone)]
pub struct SubImage {
    image: Image,
    pos: Point,
    size: Point,
}

#[derive(Clone)]
enum ImageKind {
    Empty,
    Collected {
        sub_images: Vec<SubImage>,
    },
    Composed {
        tex_coords: [[TexCoord; 4]; 4],
        grid_size: [f32; 2],
    },
    ComposedVertical {
        tex_coords: [[TexCoord; 4]; 2],
        grid_size: [f32; 2],
    },
    ComposedHorizontal {
        tex_coords: [[TexCoord; 2]; 4],
        grid_size: [f32; 2],
    },
    Solid,
    Simple {
        tex_coords: [TexCoord; 2],
        base_size: [f32; 2],
        fill: ImageFill,
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
    pub scale: f32,
}

#[derive(Clone)]
pub struct Image {
    texture: TextureHandle,
    color: Color,
    kind: ImageKind,
    base_size: Point,
}

impl Image {
    pub(crate) fn create_empty() -> Image {
        Image {
            texture: TextureHandle::default(),
            color: Color::white(),
            kind: ImageKind::Empty,
            base_size: Point::default(),
        }
    }

    pub fn texture(&self) -> TextureHandle { self.texture }

    pub fn base_size(&self) -> Point { self.base_size }

    pub(crate) fn draw<D: DrawList>(
        &self,
        draw_list: &mut D,
        params: ImageDrawParams,
    ) {
        match &self.kind {
            ImageKind::Empty => (),
            ImageKind::Collected { sub_images } => {
                for sub_image in sub_images {
                    let image = &sub_image.image;
                    let x = if sub_image.pos.x >= 0.0 {
                        params.pos[0] + sub_image.pos.x
                    } else {
                        params.pos[0] + params.size[0] + sub_image.pos.x
                    };

                    let y = if sub_image.pos.y >= 0.0 {
                        params.pos[1] + sub_image.pos.y
                    } else {
                        params.pos[1] + params.size[1] + sub_image.pos.y
                    };

                    let w = if sub_image.size.x > 0.0 {
                        sub_image.size.x
                    } else {
                        params.size[0] + sub_image.size.x
                    };

                    let h = if sub_image.size.y > 0.0 {
                        sub_image.size.y
                    } else {
                        params.size[1] + sub_image.size.y
                    };

                    let clip = params.clip.min(Rect::new(Point::new(x, y), Point::new(w, h)));

                    let sub_params = ImageDrawParams {
                        pos: [x, y],
                        size: [w, h],
                        anim_state: params.anim_state,
                        clip,
                        time_millis: params.time_millis,
                        scale: params.scale,
                    };

                    image.draw(draw_list, sub_params);
                }
            },
            ImageKind::Composed { tex_coords, grid_size } => {
                self.draw_composed(
                    draw_list,
                    tex_coords,
                    [grid_size[0] * params.scale, grid_size[1] * params.scale],
                    [params.pos[0] * params.scale, params.pos[1] * params.scale],
                    [params.size[0] * params.scale, params.size[1] * params.scale],
                    params.clip * params.scale
                );
            },
            ImageKind::ComposedVertical { tex_coords, grid_size } => {
                self.draw_composed_vertical(
                    draw_list,
                    tex_coords,
                    [grid_size[0] * params.scale, grid_size[1] * params.scale],
                    [params.pos[0] * params.scale, params.pos[1] * params.scale],
                    [params.size[0] * params.scale, params.size[1] * params.scale],
                    params.clip * params.scale
                )
            },
            ImageKind::ComposedHorizontal { tex_coords, grid_size } => {
                self.draw_composed_horizontal(
                    draw_list,
                    tex_coords,
                    [grid_size[0] * params.scale, grid_size[1] * params.scale],
                    [params.pos[0] * params.scale, params.pos[1] * params.scale],
                    [params.size[0] * params.scale, params.size[1] * params.scale],
                    params.clip * params.scale
                )
            },
            ImageKind::Solid => {
                let clip = params.clip * params.scale;
                self.draw_solid(
                    draw_list,
                    [params.pos[0] * params.scale, params.pos[1] * params.scale],
                    [params.size[0] * params.scale, params.size[1] * params.scale],
                    clip,
                );
            }
            ImageKind::Simple { tex_coords, base_size, fill } => {
                let clip = params.clip * params.scale;
                match fill {
                    ImageFill::None => {
                        self.draw_simple(
                            draw_list,
                            tex_coords,
                            [params.pos[0] * params.scale, params.pos[1] * params.scale],
                            [base_size[0] * params.scale, base_size[1] * params.scale],
                            clip,
                        );
                    }, ImageFill::Stretch => {
                        self.draw_simple(
                            draw_list,
                            tex_coords,
                            [params.pos[0] * params.scale, params.pos[1] * params.scale],
                            [params.size[0] * params.scale, params.size[1] * params.scale],
                            clip,
                        );
                    }, ImageFill::Repeat => {
                        let mut y = params.pos[1];
                        loop {
                            let mut x = params.pos[0];
                            loop {
                                self.draw_simple(
                                    draw_list,
                                    tex_coords,
                                    [x * params.scale, y * params.scale],
                                    [base_size[0] * params.scale, base_size[1] * params.scale],
                                    clip,
                                );

                                x += base_size[0];
                                if x >= params.size[0] + params.pos[0] { break; }
                            }

                            y += base_size[1];
                            if y >= params.size[1] + params.pos[1] { break; }
                        }
                    }
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
        def: &ImageDefinition,
        texture: &TextureData,
        others: &HashMap<String, Image>,
        scale: f32,
    )-> Result<Image, Error> {
        let base_size;
        let kind = match &def.kind {
            ImageDefinitionKind::Alias { .. } | ImageDefinitionKind::Group { .. } => unreachable!(),
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

                let grid_size = [grid_size[0] as f32 * scale, grid_size[1] as f32 * scale];
                base_size = Point::new(grid_size[0] * 3.0, grid_size[1] * 3.0);
                ImageKind::Composed { tex_coords, grid_size }
            },
            ImageDefinitionKind::ComposedHorizontal { grid_size_horiz, position } => {
                let mut tex_coords = [[TexCoord::default(); 2]; 4];
                for y in 0..2 {
                    #[allow(clippy::needless_range_loop)]
                    for x in 0..4 {
                        let x_val = position[0] + x as u32 * grid_size_horiz[0];
                        let y_val = position[1] + y as u32 * grid_size_horiz[1];
                        tex_coords[x][y] = texture.tex_coord(x_val, y_val);
                    }
                }
                
                let grid_size = [grid_size_horiz[0] as f32 * scale, grid_size_horiz[1] as f32 * scale];
                base_size = Point::new(grid_size[0] * 3.0, grid_size[1]);
                ImageKind::ComposedHorizontal { tex_coords, grid_size }
            },
            ImageDefinitionKind::ComposedVertical { grid_size_vert, position } => {
                let mut tex_coords = [[TexCoord::default(); 4]; 2];
                for y in 0..4 {
                    #[allow(clippy::needless_range_loop)]
                    for x in 0..2 {
                        let x_val = position[0] + x as u32 * grid_size_vert[0];
                        let y_val = position[1] + y as u32 * grid_size_vert[1];
                        tex_coords[x][y] = texture.tex_coord(x_val, y_val);
                    }
                }
                
                let grid_size = [grid_size_vert[0] as f32 * scale, grid_size_vert[1] as f32 * scale];
                base_size = Point::new(grid_size[0] * 3.0, grid_size[1]);
                ImageKind::ComposedVertical { tex_coords, grid_size }
            },
            ImageDefinitionKind::Solid { .. } => {
                base_size = Point::new(1.0, 1.0);
                ImageKind::Solid
            },
            ImageDefinitionKind::Simple { size, position, fill } => {
                let tex1 = texture.tex_coord(position[0], position[1]);
                let tex2 = texture.tex_coord(position[0] + size[0], position[1] + size[1]);
                base_size = Point::new(size[0] as f32 * scale, size[1] as f32 * scale);
                ImageKind::Simple { tex_coords: [tex1, tex2], base_size: base_size.into(), fill: *fill }
            },
            ImageDefinitionKind::Collected { sub_images } => {
                let mut size = Point::default();
                let mut images_out = Vec::new();
                for (id, sub_image_def) in sub_images {
                    let image = find_image_in_set(image_id, others, &id)?;
                    size = size.max(image.base_size);

                    images_out.push(SubImage {
                        image,
                        pos: Point::new(sub_image_def.position[0] as f32 * scale, sub_image_def.position[1] as f32 * scale),
                        size: Point::new(sub_image_def.size[0] as f32 * scale, sub_image_def.size[1] as f32 * scale),
                    })
                }

                base_size = size;
                ImageKind::Collected { sub_images: images_out }
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
                ImageKind::Timed { frame_time_millis: *frame_time_millis, frames: frames_out, once: *once }
            },
            ImageDefinitionKind::Animated { states } => {
                let mut size = Point::default();
                let mut states_out: Vec<(AnimState, Image)> = Vec::new();
                for (state, id) in states {
                    let image = find_image_in_set(image_id, others, &id)?;
                    size = image.base_size;
                    states_out.push((*state, image));
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

    fn draw_solid<D: DrawList>(
        &self,
        draw_list: &mut D,
        pos: [f32; 2],
        size: [f32; 2],
        clip: Rect,
    ) {
        draw_list.push_rect(
            [pos[0], pos[1]],
            [size[0], size[1]],
            [TexCoord::default(), TexCoord::default()],
            self.color,
            clip,
        )
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

    fn draw_composed_horizontal<D: DrawList>(
        &self,
        draw_list: &mut D,
        tex: &[[TexCoord; 2]; 4],
        grid_size: [f32; 2],
        pos: [f32; 2],
        size: [f32; 2],
        clip: Rect,
    ) {
        draw_list.push_rect(
            pos,
            [grid_size[0], size[1]],
            [tex[0][0], tex[1][1]],
            self.color,
            clip,
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_rect(
                [pos[0] + grid_size[0], pos[1]],
                [size[0] - 2.0 * grid_size[0], size[1]],
                [tex[1][0], tex[2][1]],
                self.color,
                clip,
            );
        }

        draw_list.push_rect(
            [pos[0] + size[0] - grid_size[0], pos[1]],
            [grid_size[0], size[1]],
            [tex[2][0], tex[3][1]],
            self.color,
            clip,
        );
    }

    fn draw_composed_vertical<D: DrawList>(
        &self,
        draw_list: &mut D,
        tex: &[[TexCoord; 4]; 2],
        grid_size: [f32; 2],
        pos: [f32; 2],
        size: [f32; 2],
        clip: Rect,
    ) {
        draw_list.push_rect(
            pos,
            [size[0], grid_size[1]],
            [tex[0][0], tex[1][1]],
            self.color,
            clip,
        );

        if size[1] > 2.0 * grid_size[1] {
            draw_list.push_rect(
                [pos[0], pos[1] + grid_size[1]],
                [size[0], size[1] - 2.0 * grid_size[1]],
                [tex[0][1], tex[1][2]],
                self.color,
                clip,
            );
        }

        draw_list.push_rect(
            [pos[0], pos[1] + size[1] - grid_size[1]],
            [size[0], grid_size[1]],
            [tex[0][2], tex[1][3]],
            self.color,
            clip,
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