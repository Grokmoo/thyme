use std::fs::{File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use erased_serde::Deserializer;

use crate::Error;
use crate::theme::ThemeSet;
use crate::theme_definition::ThemeDefinition;
use crate::render::{Renderer, TextureData, TextureHandle};

struct ThemeSource {
    data: Option<ThemeDefinition>,
    files: Option<ThemeSourceFiles>,
}

struct ThemeSourceFiles {
    paths: Vec<PathBuf>,
    de_func: Box<dyn Fn(&str) -> DeFuncResult>,
}

type DeFuncResult<'a> = Result<Box<dyn Deserializer<'a>>, Box<dyn std::error::Error>>;

struct ImageSource {
    data: Option<(Vec<u8>, u32, u32)>,
    file: Option<PathBuf>,
}

struct FontSource {
    font: Option<rusttype::Font<'static>>,
    data: Option<Vec<u8>>,
    file: Option<PathBuf>,
}

pub(crate) struct ResourceSet {
    // preserve ordering of images and fonts
    images: Vec<(String, ImageSource)>,
    fonts: Vec<(String, FontSource)>,
    theme: ThemeSource,
}

impl ResourceSet {
    pub(crate) fn new() -> ResourceSet {
        ResourceSet {
            images: Vec::new(),
            fonts: Vec::new(),
            theme: ThemeSource {
                data: None,
                files: None,
            },
        }
    }

    pub(crate) fn register_theme(&mut self, theme: ThemeDefinition) {
        self.theme.data = Some(theme);
        self.theme.files = None;
    }

    pub(crate) fn register_theme_from_files<E, D, F>(
        &mut self,
        paths: &[&Path],
        f: F
    ) where 
        E: 'static + std::error::Error,
        D: 'static + for<'a> serde::Deserializer<'a>,
        F: 'static + Fn(&str) -> Result<D, E>,
    {
        let boxed_fn: Box<dyn Fn(&str) -> DeFuncResult> = Box::new(move |input| {
            let result = match (f)(input) {
                Err(e) => Err(Box::new(e)),
                Ok(data) => Ok(data),
            }?;

            Ok(Box::new(Deserializer::erase(result)))
        });

        let paths: Vec<PathBuf> = paths.iter().map(|p| (*p).to_owned()).collect();

        self.theme.files = Some(ThemeSourceFiles {
            paths,
            de_func: boxed_fn,
        });
    }

    pub(crate) fn register_font_from_file(&mut self, id: String, path: &Path) {
        self.fonts.push((id, FontSource { font: None, data: None, file: Some(path.to_owned()) }));
    }

    pub(crate) fn register_font_from_data(&mut self, id: String, data: Vec<u8>) {
        self.fonts.push((id, FontSource { font: None, data: Some(data), file: None }));
    }

    pub(crate) fn register_image_from_file(&mut self, id: String, path: &Path) {
        self.images.push((id, ImageSource { data: None, file: Some(path.to_owned()) }));
    }

    pub(crate) fn register_image_from_data(&mut self, id: String, data: Vec<u8>, width: u32, height: u32) {
        self.images.push((id, ImageSource { data: Some((data, width, height)), file: None }));
    }

    /// Builds all assets and registers them with the renderer.  You must make sure all asset
    /// data is cached with [`cache_data`](#method.cache_assets) prior to calling this.
    pub(crate) fn build_assets<R: Renderer>(&mut self, renderer: &mut R, scale_factor: f32) -> Result<ThemeSet, Error> {
        let textures = self.build_images(renderer)?;
        let fonts = self.build_fonts()?;

        let theme_def = match &self.theme.data {
            None => {
                return Err(Error::Theme("Cannot build assets.  No theme specified.".to_string()));
            },
            Some(def) => def,
        };
        let themes = ThemeSet::new(theme_def, textures, fonts, renderer, scale_factor)?;

        Ok(themes)
    }

    pub(crate) fn clear_data_cache(&mut self) {
        if self.theme.files.is_some() {
            self.theme.data = None;
        }

        for (_, src) in self.images.iter_mut() {
            if src.file.is_some() {
                src.data = None;
            }
        }

        for (_, src) in self.fonts.iter_mut() {
            if src.file.is_some() {
                src.data = None;
                src.font = None;
            }
        }
    }

    pub(crate) fn cache_data(&mut self) -> Result<(), Error> {
        if self.theme.data.is_none() {
            if let Some(theme_source) = self.theme.files.as_ref() {
                let mut theme_def: Option<ThemeDefinition> = None;

                let mut theme_str = String::new();
                for path in &theme_source.paths {
                    let mut file = match File::open(path) {
                        Ok(file) => file,
                        Err(e) => return Err(Error::IO(e)),
                    };

                    theme_str.clear();
                    match file.read_to_string(&mut theme_str) {
                        Err(e) => return Err(Error::IO(e)),
                        Ok(count) => {
                            log::debug!("Read {} bytes from '{:?}' for theme.", count, path);
                        }
                    }

                    let theme_value = match (theme_source.de_func)(&theme_str) {
                        Ok(value) => value,
                        Err(e) => return Err(Error::Serde(e.to_string())),
                    };
                    
                    match theme_def.as_mut() {
                        None => {
                            theme_def = Some(match serde::Deserialize::deserialize(theme_value) {
                                Ok(theme) => theme,
                                Err(e) => return Err(Error::Serde(e.to_string())),
                            });
                        }, Some(theme) => {
                            let new_theme_def: ThemeDefinition = match serde::Deserialize::deserialize(theme_value) {
                                Ok(theme) => theme,
                                Err(e) => return Err(Error::Serde(e.to_string())),
                            };

                            theme.merge(new_theme_def);
                        }
                    }
                }

                if theme_def.is_none() {
                    return Err(Error::Theme("No valid theme was specified".to_string()));
                }

                self.theme.data = theme_def;
            }
        }

        for (id, src) in self.images.iter_mut() {
            if src.data.is_some() { continue; }
            
            // file must always be some if data is none
            let path = src.file.as_ref().unwrap();

            let image = match image::open(path) {
                Ok(image) => image.into_rgba(),
                Err(error) => return Err(Error::Image(error)),
            };

            let dims = image.dimensions();
            let data = image.into_raw();

            log::debug!("Read {} bytes from '{:?}' for image '{}'", data.len(), path, id);

            src.data = Some((data, dims.0, dims.1));
        }

        for (id, src) in self.fonts.iter_mut() {
            if src.font.is_some() { continue; }
            
            let data = if let Some(data) = src.data.as_ref() {
                data.clone()
            } else {
                // file must always be some if data is none
                let path = src.file.as_ref().unwrap();
                let data = match std::fs::read(path) {
                    Ok(data) => data,
                    Err(error) => return Err(Error::IO(error)),
                };

                log::debug!("Read {} bytes from '{:?}' for font '{}'", data.len(), path, id);

                let result = data.clone();
                src.data = Some(data);
                result
            };

            let font = match rusttype::Font::try_from_vec(data) {
                Some(font) => font,
                None => return Err(
                    Error::FontSource(format!("Unable to parse '{}' as ttf", id))
                )
            };

            log::debug!("Created rusttype font from '{}'", id);

            src.font = Some(font);
        }

        Ok(())
    }

    fn build_fonts(&mut self) -> Result<HashMap<String, crate::font::FontSource>, Error> {
        let mut output = HashMap::new();

        for (id, source) in self.fonts.iter_mut() {
            let font = source.font.take().unwrap();
            output.insert(id.to_string(), crate::font::FontSource { font });
        }

        Ok(output)
    }

    fn build_images<R: Renderer>(&self, renderer: &mut R) -> Result<HashMap<String, TextureData>, Error> {
        let mut handle = TextureHandle::default();

        let mut output = HashMap::new();
        for (id, source) in self.images.iter() {
            let (tex_data, width, height) = source.data.as_ref().unwrap();
            let dims = (*width, *height);
            let tex_data = renderer.register_texture(handle, &tex_data, dims)?;
            output.insert(id.to_string(), tex_data);

            handle = handle.next();
        }

        Ok(output)
    }
}