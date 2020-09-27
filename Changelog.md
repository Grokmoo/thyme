# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2020-09-26
### Added
- Assets can now be read from files or supplied directly.
- Optional Live Reload support for theme, image, and font files.
- Hot swapping between themes and several new example themes.
- More flexible theme file merging from multiple sources.
- More widgets - tooltip, spinner, tree.
- Improved documentation and added many code examples.
- "Children" size from attribute.
- Image aliases and "empty" image for overriding purposes

### Changed
- Improved asset organization for the examples.
- "from" theme references can now be resolved relative to the current theme as well as absolutely.
- Input fields may specify an initial value
- Windows may now optionally specify their title in code.
- Improved querying persistent state.

### Fixed
- Modal widgets will always want the mouse.
- Combo boxes should now position and clip correctly and handle non-copy types.
- Fixed several render group ordering issues
- Fixed scaling for collected images

## [0.1.0] - 2020-09-01
### Added
- Initial release with theming, HiDPI support, TTF Fonts, Glium and wgpu based backends.