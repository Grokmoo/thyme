# Thyme - Themable Immediate Mode GUI

Thyme is a Graphical User Interface (GUI) library written in pure, safe, Rust.  All widgets are rendered using image sources, instead of the line art more commonly used by other Immediate Mode GUIs.  The image definitions, fonts, and style attributes are all specified in a unified theme.  This is generally drawn from a file, but any [Serde](https://serde.rs/) compatible source should work.

Thyme produces a set of Draw Lists which are sent to a swappable graphics backend - currently [Glium](https://github.com/glium/glium) is supported.  The I/O backend is also swappable - currently [winit](https://github.com/rust-windowing/winit) is supported.  Fonts are rendered to a texture on the GPU using [rusttype](https://github.com/redox-os/rusttype).  

Performance is already acceptable for most use cases, with the complete cycle of generating the widget tree, creating the draw data, and rendering taking less than 1 ms for moderately complex UIs.  This is without any significant effort made towards optimization.

## License
[License]: #license

Licensed under Apache License, Version 2.0, ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)

### License of contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.