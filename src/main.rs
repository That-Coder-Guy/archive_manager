// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;
use rfd::FileDialog;
use slint::SharedString;

mod archive;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    archive::create_cache();
    let ui = AppWindow::new()?;

    // Open folder dialog callback body
    ui.on_open_folder_dialog(|| {
        match FileDialog::new().pick_folder()
        {
            Some(folder_path) => {
                println!("Selected folder: {:?}", folder_path.display());
                return SharedString::from(folder_path.to_string_lossy().to_string());
            }
            None => {
                return SharedString::from("");
            }
        }
    });

    ui.run()?;

    return Ok(());
}