mod app;
mod model;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 680.0])
            .with_title("RO3 Translation Editor"),
        ..Default::default()
    };
    eframe::run_native(
        "RO3 Translation Editor",
        options,
        Box::new(|cc| Ok(Box::new(app::EditorApp::new(cc)))),
    )
}
