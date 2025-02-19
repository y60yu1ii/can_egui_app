#![windows_subsystem = "windows"]

mod canbus;
mod ui_components;

use ui_components::MyApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "CAN Bus 控制 App",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))), // ✅ 用 Ok() 包裝
    )?;

    Ok(())
}
