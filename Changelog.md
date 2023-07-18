# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2023-07-17
### Changed
- Improved handling of custom variables across widgets
- Better formatting and spacing for table widgets
- More flexible benchmarking
- Configurable build options, including tooltip time and line scroll which can be set manually or via AppBuilder
- Better tooltip layout and edge of screen positioning
- Improved render group ordering and ability to specify always top / always bottom

### Added
- Handy macro `set_variables` to set multiple variables on a text field
- Text elements within text areas can now specify a text color
- `force_hover` and `force_pressed` methods when building widgets
- An optional `edit` method is now available to improve chaining of certain types of control logic
- AppBuilder now supports GL renderer
- Optionally specify `height` and `width` instead of `size` in theme
- New multiline text widget with simple rendering (as opposed to text area)
- if / else statements supported in text area definition
- Method to obtain mouse position from the context
- Vertical (in addition to horizontal) progress bars
- Support for saving and loading of persistent state to a file / other output
- Text layout option based on text width for single line widgets
- Wrapping spinner widget
- Can now specify a dynamic / theme based image color attribute
- Now fully handle left / right / middle clicks

### Fixed
- Fixed text area end of line behavior in some cases
- Color space issues for GL renderer
- Custom ints are now parsed correctly

## [0.6.0] - 2021-03-31
### Changed
- Improved the theme definitions for the demo example

### Added
- A third renderer backend, using straight OpenGL, is now available
- Support for dynamic variable substitution in text fields
- A textbox widget that parses a subset of Markdown, including strong / emphasis, headers, and tables
- Added ability to define a theme without any actual image sources, and a demo example
- Image aliases now can be used in the theme definition to avoid repitition
- Multiple simple images can now be quickly defined using image groups
- Method to query the current parent Widget bounds
- Image colors now support transparency / alpha

### Fixed
- The first example in the docs actually compiles now

## [0.5.0] - 2020-12-01
### Changed
- Font character cache texture is more appropriately sized
- Example themes are better organized

### Added
- Support for user specified arbitrary character ranges in fonts
- AppBuilder helper class allow users to set up a basic app in very few lines of code
- Thyme images can be defined without requiring an actual image on disk

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
