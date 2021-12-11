use std::{path::Path};

use crate::{Error, Context};
use crate::{resource::ResourceSet};
use crate::theme_definition::{ThemeDefinition};
use crate::render::{Renderer, IO};

/// Global options that may be specified when building the Thyme context with
/// [`ContextBuilder`](struct.ContextBuilder.html).  These options
/// cannot be changed afterwards.
pub struct BuildOptions {
    /// Whether to enable background file monitoring for live reload.  Note that
    /// to actually make use of this feature, you will need to call
    /// [`check_live_reload`](struct.Context.html#method.check_live_reload), typically
    /// once between each frame.  The default value is `true`.
    pub enable_live_reload: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            enable_live_reload: true,
        }
    }
}

/// Structure to register resources and ultimately build the main Thyme [`Context`](struct.Context.html).
///
/// You pass resources to it to register them with Thyme.  Once this process is complete, call
/// [`build`](struct.ContextBuilder.html#method.build) to create your [`Context`](struct.Context.html).
pub struct ContextBuilder {
    resources: ResourceSet,
}

impl ContextBuilder {
    /**
    Creates a new `ContextBuilder`, using the default [`BuildOptions`](struct.BuildOptions.html)

    # Example
    ```no_run
        let mut context_builder = thyme::ContextBuilder::with_defaults();
        context_builder.register_theme(theme)?;
        ...
    ```
    **/
    pub fn with_defaults() -> ContextBuilder {
        ContextBuilder::new(BuildOptions::default())
    }

    /// Creates a new `ContextBuilder`, using the specified [`BuildOptions`](struct.BuildOptions.html)
    pub fn new(options: BuildOptions) -> ContextBuilder {
        ContextBuilder {
            resources: ResourceSet::new(options.enable_live_reload),
        }
    }

    /// Sets the theme for this context.  The theme for your UI will be deserialized from
    /// `theme`.  For example, `theme` could be a [`serde_json Value`](https://docs.serde.rs/serde_json/value/enum.Value.html) or
    /// [`serde_yaml Value`](https://docs.serde.rs/serde_yaml/enum.Value.html).  See [`the crate root`](index.html) for a
    /// discussion of the theme format.  If this method is called multiple times, only the last
    /// theme is used
    pub fn register_theme<'a, T: serde::Deserializer<'a>>(&mut self, theme: T) -> Result<(), T::Error> {
        log::debug!("Registering theme");
        
        let theme_def: ThemeDefinition = serde::Deserialize::deserialize(theme)?;
        self.resources.register_theme(theme_def);
        Ok(())
    }

    /// Sets the theme for this context by reading from the file at the specified `path`.  The file is
    /// deserialized as serde YAML files.  See [`register_theme`](#method.register_theme)
    pub fn register_theme_from_file(
        &mut self,
        path: &Path,
    ) -> Result<(), Error> {
        log::debug!("Reading theme from file: '{:?}'", path);

        self.resources.register_theme_from_files(&[path]);

        Ok(())
    }

    /// Sets the theme for this context by reading from the specified list of files.  The files are each read into a
    /// string and then concatenated together.  The string is then deserialized as serde YAML.  See
    /// [`register_theme`](#method.register_theme)
    pub fn register_theme_from_files(
        &mut self,
        paths: &[&Path],
    ) -> Result<(), Error> {
        log::debug!("Reading theme from files: '{:?}'", paths);

        self.resources.register_theme_from_files(paths);
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

    /// Consumes this builder and releases the borrows on the [`Renderer`](trait.Renderer.html) and [`IO`](trait.IO.html),
    /// so they can be used further.  Builds a [`Context`](struct.Context.html).
    pub fn build<R: Renderer, I: IO>(mut self, renderer: &mut R, io: &mut I) -> Result<Context, Error> {
        log::info!("Building Thyme Context");
        let scale_factor = io.scale_factor();
        let display_size = io.display_size();

        self.resources.cache_data()?;
        let themes = self.resources.build_assets(renderer, scale_factor)?;
        Ok(Context::new(self.resources, themes, display_size, scale_factor))
    }
}