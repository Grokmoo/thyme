use std::{path::Path};

use crate::{Error, Context};
use crate::{resource::ResourceSet};
use crate::theme_definition::{ThemeDefinition};
use crate::render::{Renderer, IO};

/// Structure to register resources and ultimately build the main Thyme [`Context`](struct.Context.html).
///
/// This will hold references to your chosen [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html).
/// You pass resources to it to register them with Thyme.  Once this process is complete, call
/// [`build`](struct.ContextBuilder.html#method.build) to create your [`Context`](struct.Context.html).
pub struct ContextBuilder<'a, R: Renderer, I: IO> {
    renderer: &'a mut R,
    io: &'a mut I,
    resources: ResourceSet,
}

impl<'a, R: Renderer, I: IO> ContextBuilder<'a, R, I> {
    /// Creates a new `ContextBuilder`, from the specified [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html).
    pub fn new(renderer: &'a mut R, io: &'a mut I) -> ContextBuilder<'a, R, I> {
        ContextBuilder {
            renderer,
            io,
            resources: ResourceSet::new(),
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
        self.resources.register_theme(theme_def);
        Ok(())
    }

    /// Sets the theme for this context by reading from the file at the specified `path`.  The files are first
    /// read to a string and then passed to the function `f`, which returns a serde Deserializable object.  That
    /// object is then deserialized as the theme.  See [`register_theme`](#method.register_theme)
    pub fn register_theme_from_file<T, E, F>(
        &mut self,
        path: &Path,
        f: F,
    ) -> Result<(), Error> where
        T: 'static + for<'de> serde::Deserializer<'de>,
        E: 'static + std::error::Error,
        F: 'static + Fn(&str) -> Result<T, E>
    {
        log::debug!("Reading theme from file: '{:?}'", path);

        self.resources.register_theme_from_files(&[path], f);

        Ok(())
    }

    /// Sets the theme for this context by reading from the specified list of files.  The files are each read into a
    /// string and then concatenated together.  The string is passed to the function `f` which returns a serde
    /// Deserializable object, which is finally deserialized into the theme.  See
    /// [`register_theme`](#method.register_theme)
    pub fn register_theme_from_files<T, E, F>(
        &mut self,
        paths: &[&Path],
        f: F,
    ) -> Result<(), Error> where
        T: 'static + for<'de> serde::Deserializer<'de>,
        E: 'static + std::error::Error,
        F: 'static + Fn(&str) -> Result<T, E>
    {
        log::debug!("Reading theme from files: '{:?}'", paths);

        self.resources.register_theme_from_files(paths, f);
        Ok(())
    }

    /// Registers the font data located in the file at the specified `path` with Thyme via the specified `id`.
    /// See [`register_font`](#method.register_font)
    pub fn register_font_from_file<T: Into<String>>(
        &mut self,
        id: T,
        path: &Path,
    ) {
        let id = id.into();
        log::debug!("Reading font source '{}' from file: '{:?}'", id, path);
        self.resources.register_font_from_file(id, path);
    }

    /// Registers the font data for use with Thyme via the specified `id`.  The `data` must consist
    /// of the full binary for a valid TTF or OTF file.
    /// Once the font has been registered, it can be accessed in your theme file via the font `source`.
    pub fn register_font<T: Into<String>>(
        &mut self,
        id: T,
        data: Vec<u8>
    ) {
        let id = id.into();
        log::debug!("Registering font source '{}'", id);
        self.resources.register_font_from_data(id, data);
    }

    /// Reads a texture from the specified image file.  See [`register_texture`](#method.register_texture).
    /// Requires you to enable the `image` feature in `Cargo.toml` to enable the dependancy on the
    /// [`image`](https://github.com/image-rs/image) crate.
    #[cfg(feature="image")]
    pub fn register_texture_from_file<T: Into<String>>(
        &mut self,
        id: T,
        path: &Path,
    ) {
        let id = id.into();
        log::debug!("Reading texture '{}' from file: '{:?}'", id, path);
        self.resources.register_image_from_file(id, path);
    }

    /// Registers the image data for use with Thyme via the specified `id`.  The `data` must consist of
    /// raw binary image data in RGBA format, with 4 bytes per pixel.  The data must start at the
    /// bottom-left hand corner pixel and progress left-to-right and bottom-to-top.  `data.len()` must
    /// equal `dimensions.0 * dimensions.1 * 4`
    /// Once the image has been registered, it can be accessed in your theme file via the image `source`.
    pub fn register_texture<T: Into<String>>(
        &mut self,
        id: T,
        data: Vec<u8>,
        dimensions: (u32, u32),
    ) {
        let id = id.into();
        log::debug!("Registering texture '{}'", id);
        self.resources.register_image_from_data(id, data, dimensions.0, dimensions.1);
    }

    /// Consumes this builder and releases the borrows on the [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html), so they can
    /// be used further.  Builds a [`Context`](struct.Context.html).
    pub fn build(mut self) -> Result<Context, Error> {
        log::info!("Building Thyme Context");
        let scale_factor = self.io.scale_factor();
        let display_size = self.io.display_size();

        self.resources.cache_data()?;
        let themes = self.resources.build_assets(self.renderer, scale_factor)?;
        Ok(Context::new(self.resources, themes, display_size, scale_factor))
    }
}