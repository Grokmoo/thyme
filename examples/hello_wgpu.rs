fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = thyme::AppBuilder::new()
        .with_logger()
        .with_title("Thyme wgpu Demo")
        .with_window_size(1280.0, 720.0)
        .with_base_dir("examples/data")
        .with_theme_files(&["themes/base.yml", "themes/pixel.yml"])
        .with_font_dir("fonts")
        .with_image_dir("images")
        .build_wgpu()?;

    app.main_loop(|ui| {
        ui.window("window", |ui| {
            ui.gap(20.0);
    
            ui.button("label", "Hello, World!");
        });
    });
}