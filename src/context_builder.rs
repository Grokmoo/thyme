use std::{path::Path, fs::File, io::Read};
use std::collections::{HashMap};

use crate::{Error, Context};
use crate::theme::ThemeSet;
use crate::theme_definition::{ThemeDefinition};
use crate::render::{Renderer, IO, TextureData, TextureHandle};
use crate::font::FontSource;


/// Structure to register resources and ultimately build the main Thyme [`Context`](struct.Context.html).
///
/// This will hold references to your chosen [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html).
/// You pass resources to it to register them with Thyme.  Once this process is complete, call
/// [`build`](struct.ContextBuilder.html#method.build) to create your [`Context`](struct.Context.html).
pub struct ContextBuilder<'a, R: Renderer, I: IO> {
    renderer: &'a mut R,
    io: &'a mut I,
    font_sources: HashMap<String, FontSource>,
    textures: HashMap<String, TextureData>,
    next_texture_handle: TextureHandle,
    theme_def: Option<ThemeDefinition>,
}

impl<'a, R: Renderer, I: IO> ContextBuilder<'a, R, I> {
    /// Creates a new `ContextBuilder`, from the specified [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html).
    pub fn new(renderer: &'a mut R, io: &'a mut I) -> ContextBuilder<'a, R, I> {
        renderer.clear_assets();

        ContextBuilder {
            renderer,
            io,
            font_sources: HashMap::new(),
            textures: HashMap::new(),
            next_texture_handle: TextureHandle::default(),
            theme_def: None,
        }
    }

    /// Sets the theme for this context.  The theme for your UI will be deserialized from
    /// `theme`.  For example, `theme` could be a [`serde_json Value`](https://docs.serde.rs/serde_json/value/enum.Value.html) or
    /// [`serde_yaml Value`](https://docs.serde.rs/serde_yaml/enum.Value.html).  See [`the crate root`](index.html) for a
    /// discussion of the theme format.  If this method is called multiple times, only the last
    /// theme is used
    pub fn register_theme<T: serde::Deserializer<'a>>(&mut self, theme: T) -> Result<(), T::Error> {
        log::debug!("Registering theme");
        
        let theme_def: ThemeDefinition = serde::Deserialize::deserialize(theme)?;
        self.theme_def = Some(theme_def);
        Ok(())
    }

    /// Sets the theme for this context by reading from the file at the specified `path`.  The files are first
    /// read to a string and then passed to the function `f`, which returns a serde Deserializable object.  That
    /// object is then deserialized as the theme.  See [`register_theme`](#method.register_theme)
    pub fn register_theme_from_file<T: serde::Deserializer<'a>, E: std::error::Error + 'static, F: Fn(&str) -> Result<T, E>>(
        &mut self,
        path: &Path,
        f: F,
    ) -> Result<(), Error> {
        log::debug!("Reading theme from file: '{:?}'", path);

        let theme = match std::fs::read_to_string(path) {
            Ok(data) => data,
            Err(e) => return Err(Error::IO(e)),
        };

        self.register_theme_from_str(&theme, f)
    }

    /// Sets the theme for this context by reading from the specified list of files.  The files are each read into a
    /// string and then concatenated together.  The string is passed to the function `f` which returns a serde
    /// Deserializable object, which is finally deserialized into the theme.  See
    /// [`register_theme`](#method.register_theme)
    pub fn register_theme_from_files<T: serde::Deserializer<'a>, E: std::error::Error + 'static, F: Fn(&str) -> Result<T, E>>(
        &mut self,
        paths: &[&Path],
        f: F,
    ) -> Result<(), Error> {
        log::debug!("Reading theme from files: '{:?}'", paths);

        let mut theme = String::new();
        for path in paths {
            let mut file = match File::open(path) {
                Ok(file) => file,
                Err(e) => return Err(Error::IO(e)),
            };

            if let Err(e) = file.read_to_string(&mut theme) {
                return Err(Error::IO(e));
            }
        }

        self.register_theme_from_str(&theme, f)
    }

    fn register_theme_from_str<T: serde::Deserializer<'a>, E: std::error::Error + 'static, F: Fn(&str) -> Result<T, E>>(
        &mut self,
        theme: &str,
        f: F,
    ) -> Result<(), Error> {
        let theme_value: T = match f(&theme) {
            Ok(value) => value,
            Err(e) => return Err(Error::Serde(e.to_string())),
        };

        let theme_def: ThemeDefinition = match serde::Deserialize::deserialize(theme_value) {
            Ok(theme) => theme,
            Err(e) => return Err(Error::Serde(e.to_string())),
        };

        self.theme_def = Some(theme_def);
        Ok(())
    }

    /// Registers the font data located in the file at the specified `path` with Thyme via the specified `id`.
    /// See [`register_font`](#method.register_font)
    pub fn register_font_from_file<T: Into<String>>(
        &mut self,
        id: T,
        path: &Path,
    ) -> Result<(), Error> {
        let id = id.into();
        log::debug!("Reading font source '{}' from file: '{:?}'", id, path);

        let data = match std::fs::read(path) {
            Ok(data) => data,
            Err(error) => return Err(Error::IO(error)),
        };
        
        self.register_font(id, data)
    }

    /// Registers the font data for use with Thyme via the specified `id`.  The `data` must consist
    /// of the full binary for a valid TTF or OTF file.
    /// Once the font has been registered, it can be accessed in your theme file via the font `source`.
    pub fn register_font<T: Into<String>>(
        &mut self,
        id: T,
        data: Vec<u8>
    ) -> Result<(), Error> {
        let id = id.into();
        log::debug!("Registering font source '{}'", id);

        let font = match rusttype::Font::try_from_vec(data) {
            Some(font) => font,
            None => return Err(
                Error::FontSource(format!("Unable to parse '{}' as ttf", id))
            )
        };
        self.font_sources.insert(id, FontSource { font });

        Ok(())
    }

    /// Reads a texture from the specified image file.  See [`register_texture`](#method.register_texture).
    /// Requires you to enable the `image` feature in `Cargo.toml` to enable the dependancy on the
    /// [`image`](https://github.com/image-rs/image) crate.
    #[cfg(feature="image")]
    pub fn register_texture_from_file<T: Into<String>>(
        &mut self,
        id: T,
        path: &Path,
    ) -> Result<(), Error> {
        let id = id.into();
        log::debug!("Reading texture '{}' from file: '{:?}'", id, path);

        let image = match image::open(path) {
            Ok(image) => image.into_rgba(),
            Err(error) => return Err(Error::Image(error)),
        };

        let dims = image.dimensions();
        self.register_texture(id, &image.into_raw(), dims)
    }

    /// Registers the image data for use with Thyme via the specified `id`.  The `data` must consist of
    /// raw binary image data in RGBA format, with 4 bytes per pixel.  The data must start at the
    /// bottom-left hand corner pixel and progress left-to-right and bottom-to-top.  `data.len()` must
    /// equal `dimensions.0 * dimensions.1 * 4`
    /// Once the image has been registered, it can be accessed in your theme file via the image `source`.
    pub fn register_texture<T: Into<String>>(
        &mut self,
        id: T,
        data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<(), Error> {
        let id = id.into();
        log::debug!("Registering texture '{}'", id);

        let handle = self.next_texture_handle;
        let data = self.renderer.register_texture(handle, data, dimensions)?;
        self.textures.insert(id, data);
        self.next_texture_handle = handle.next();

        Ok(())
    }

    /// Consumes this builder and releases the borrows on the [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html), so they can
    /// be used further.  Builds a [`Context`](struct.Context.html).
    pub fn build(self) -> Result<Context, Error> {
        let definition = match self.theme_def {
            None => {
                return Err(Error::Theme(
                    "Cannot build context.  No theme specified.".to_string()
                ));
            },
            Some(def) => def,
        };

        log::info!("Building Thyme Context");
        let scale_factor = self.io.scale_factor();
        let display_size = self.io.display_size();
        let textures = self.textures;
        let fonts = self.font_sources;
        let themes = ThemeSet::new(definition, textures, fonts, self.renderer, scale_factor)?;
        Ok(Context::new(themes, display_size, scale_factor))
    }
}