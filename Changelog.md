# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2020-10-18
### Changed
- Improved performance of wgpu and glium backends.
- wgpu and Glium examples should now be as similar as possible.
- Upgraded winit to 0.23.

### Fixed
- unparent method on WidgetBuilder now works correctly with size_from Children.
- Tooltip positions is limited to inside the app window / screen.
- display_size method on the UI Frame now correctly returns its result in logical pixels.
- Cleaned up border issues in the "pixels" theme.
- Tooltips will correctly render on top of all other render groups using the new always_top attribute.
- The Demo apps will now render at a consistent 60 frames per second.

### Added
- Keyboard modifers state is now tracked and accessible via the UI Frame.
- screen_pos attribute may now be specified in the theme.
- wants_mouse can now be obtained in the UI Frame as well as from the Context.
- Simple tooltips can be created via the theme or as a single call in WidgetBuilder.
- Expose wants_keyboard to let the client app know if Thyme is using the keyboard input on a given frame.

## [0.3.0] - 2020-09-28
### Changed
- Wgpu backend now takes an Arc instead of Rc.
- Show fewer log messages in the examples.

### Fixed
- Cleaned up docs links and typos.
- Glium and wgpu display fonts consistently
- Glium and wgpu do sRGB conversion consistently

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
