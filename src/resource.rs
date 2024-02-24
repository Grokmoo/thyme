use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{atomic::{AtomicBool, Ordering}, mpsc::{Receiver, channel}};

use notify::{Watcher, RecommendedWatcher, RecursiveMode, watcher, DebouncedEvent};

use crate::Error;
use crate::theme::ThemeSet;
use crate::theme_definition::ThemeDefinition;
use crate::render::{Renderer, TextureData, TextureHandle};

static RELOAD_THEME: AtomicBool = AtomicBool::new(false);

struct ThemeSource {
    data: Option<ThemeDefinition>,
    files: Option<Vec<PathBuf>>,
}

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

    watcher: Option<RecommendedWatcher>,
}

impl ResourceSet {
    pub(crate) fn new(enable_live_reload: bool) -> ResourceSet {
        let (tx, rx) = channel();

        let watcher = if enable_live_reload {
            match watcher(tx, Duration::from_secs(1)) {
                Err(e) => {
                    log::error!("Unable to initialize file watching for live-reload:");
                    log::error!("{}", e);
                    None
                }, Ok(watcher) => Some(watcher),
            }
        } else {
            None
        };

        if watcher.is_some() {
            std::thread::spawn(move || watcher_loop(rx) );
        }

        ResourceSet {
            images: Vec::new(),
            fonts: Vec::new(),
            theme: ThemeSource {
                data: None,
                files: None,
            },
            watcher,
        }
    }

    fn remove_path_from_watcher(&mut self, path: &Path) {
        if let Some(watcher) = self.watcher.as_mut() {
            if let Err(e) = watcher.unwatch(path) {
                log::warn!("Unable to watch path: {:?}", path);
                log::warn!("{}", e);
            }
        }
    }

    fn add_path_to_watcher(&mut self, path: &Path) {
        if let Some(watcher) = self.watcher.as_mut() {
            log::info!("Watching {:?}", path);
            if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
                log::warn!("Unable to unwatch path: {:?}", path);
                log::warn!("{}", e);
            }
        }
    }

    pub(crate) fn register_theme(&mut self, theme: ThemeDefinition) {
        self.theme.data = Some(theme);
        self.theme.files = None;
    }

    pub(crate) fn register_theme_from_files(
        &mut self,
        paths: &[&Path],
    ) {
        let mut paths_out: Vec<PathBuf> = Vec::new();
        for path in paths {
            self.add_path_to_watcher(path);
            paths_out.push((*path).to_owned());
        }

        self.theme.files = Some(paths_out);
    }

    pub(crate) fn register_font_from_file(&mut self, id: String, path: &Path) {
        self.add_path_to_watcher(path);
        self.fonts.push((id, FontSource { font: None, data: None, file: Some(path.to_owned()) }));
    }

    pub(crate) fn register_font_from_data(&mut self, id: String, data: Vec<u8>) {
        self.fonts.push((id, FontSource { font: None, data: Some(data), file: None }));
    }

    pub(crate) fn register_image_from_file(&mut self, id: String, path: &Path) {
        self.add_path_to_watcher(path);
        self.images.push((id, ImageSource { data: None, file: Some(path.to_owned()) }));
    }

    pub(crate) fn register_image_from_data(&mut self, id: String, data: Vec<u8>, width: u32, height: u32) {
        self.images.push((id, ImageSource { data: Some((data, width, height)), file: None }));
    }

    pub(crate) fn remove_theme_file(&mut self, path: &Path) {
        self.remove_path_from_watcher(path);
        if let Some(paths) = self.theme.files.as_mut() {
            paths.retain(|p| p != path);
            self.theme.data = None;
        }
    }

    pub(crate) fn add_theme_file(&mut self, path: PathBuf) {
        self.add_path_to_watcher(&path);
        if let Some(paths) = self.theme.files.as_mut() {
            paths.push(path);
            self.theme.data = None;
        }
    }

    /// Checks for a file watch change and rebuilds the theme if neccessary, clearing the data cache
    /// and reloading all data.  Will return Ok(None) if there was no change, or Err if there was
    /// a problem rebuilding the theme.
    pub(crate) fn check_live_reload<R: Renderer>(&mut self, renderer: &mut R, scale_factor: f32) -> Result<Option<ThemeSet>, Error> {
        match RELOAD_THEME.compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire) {
            Ok(true) => (),
            _ => return Ok(None),
        }

        self.clear_data_cache();
        self.cache_data()?;

        let themes = self.build_assets(renderer, scale_factor)?;

        Ok(Some(themes))
    }

    /// Builds all assets and registers them with the renderer.  You must make sure all asset
    /// data is cached with [`cache_data`](#method.cache_assets) prior to calling this.
    pub(crate) fn build_assets<R: Renderer>(&mut self, renderer: &mut R, scale_factor: f32) -> Result<ThemeSet, Error> {
        RELOAD_THEME.store(false, Ordering::Release);

        let textures = self.build_images(renderer)?;
        let fonts = self.build_fonts();

        let theme_def = match self.theme.data.as_mut() {
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
                for path in theme_source.iter() {
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

                    match theme_def.as_mut() {
                        None => {
                            theme_def = Some(match serde_yaml::from_str(&theme_str) {
                                Ok(theme) => theme,
                                Err(e) => return Err(Error::Serde(e.to_string())),
                            });
                        }, Some(theme) => {
                            let new_theme_def: ThemeDefinition = match serde_yaml::from_str(&theme_str) {
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
                Ok(image) => image.into_rgba8(),
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

    fn build_fonts(&mut self) -> HashMap<String, crate::font::FontSource> {
        let mut output = HashMap::new();

        for (id, source) in self.fonts.iter_mut() {
            let font = source.font.take().unwrap();
            output.insert(id.to_string(), crate::font::FontSource { font });
        }

        output
    }

    fn build_images<R: Renderer>(&self, renderer: &mut R) -> Result<HashMap<String, TextureData>, Error> {
        let mut output = HashMap::new();
        let mut handle = TextureHandle::default();

        // register a 1x1 pixel texture for use with minimal themes
        let tex_data = [0xff, 0xff, 0xff, 0xff];
        let tex_data = renderer.register_texture(handle, &tex_data, (1, 1))?;
        output.insert(INTERNAL_SINGLE_PIX_IMAGE_ID.to_string(), tex_data);
        handle = handle.next();
        
        for (id, source) in self.images.iter() {
            let (tex_data, width, height) = source.data.as_ref().unwrap();
            let dims = (*width, *height);
            let tex_data = renderer.register_texture(handle, tex_data, dims)?;
            output.insert(id.to_string(), tex_data);

            handle = handle.next();
        }

        Ok(output)
    }
}

pub(crate) const INTERNAL_SINGLE_PIX_IMAGE_ID: &str = "__INTERNAL_SINGLE_PIX__";

fn watcher_loop(rx: Receiver<DebouncedEvent>) {
    loop {
        match rx.recv() {
            Ok(event) => {
                use DebouncedEvent::*;
                match event {
                    NoticeWrite(..) | NoticeRemove(..) | Chmod(..) | Rescan => (),
                    Create(..) | Write(..) | Remove(..) | Rename(..) => {
                        log::info!("Received file notification: {:?}", event);
                        RELOAD_THEME.store(true, Ordering::Release);
                    },
                    Error(error, path) => {
                        log::warn!("Received file notification error for path {:?}", path);
                        log::warn!("{}", error);
                    }
                }
            },
            Err(e) => {
               log::info!("Disconnected live-reload watcher: {}", e);
            }
        }
    }
}