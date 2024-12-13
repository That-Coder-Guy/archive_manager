// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;
use rfd::FileDialog;
use slint::SharedString;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
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

/*
use walkdir::WalkDir;
use blake3::{Hasher, Hash};
use std::fs::{self};
use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let root_dir = "C:\\Users\\Owner\\Desktop\\Terabyte Backup";
    let mut image_count: i32 = 0;
    let mut map: HashMap<Hash, PathBuf> = HashMap::new();
    let mut hasher: Hasher = Hasher::new();

    for entry in WalkDir::new(root_dir)
    {
        match entry
        {
            Ok(entry) =>
            {
                if entry.file_type().is_file()
                {
                    let data: Vec<u8> = fs::read(entry.path()).unwrap();
                    hasher.update(&data);
                    match map.insert(hasher.finalize(), entry.path().to_path_buf())
                    {
                        Some(directory) =>
                        {
                            println!("{} and {} are identical", entry.path().display(), directory.as_path().display());
                        }
                        None => { }
                    }
                    image_count += 1;
                }
            }
            Err(err) =>
            {
                eprintln!("Error: {}", err);
            }
        }
    }
    println!("Image Count: {}", image_count);
}


*/