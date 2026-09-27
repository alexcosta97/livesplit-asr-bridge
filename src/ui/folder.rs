//! Opening a folder in the OS's file manager.

use std::{fs, path::Path, process::Command};

/// Opens `dir` in the file manager, creating it first if it doesn't exist
/// yet.
pub fn open_folder(dir: &Path) -> Result<(), String> {
    let describe = |error: std::io::Error| format!("Couldn't open {}: {error}", dir.display());
    fs::create_dir_all(dir).map_err(describe)?;
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut child = Command::new(program).arg(dir).spawn().map_err(describe)?;
    // Waited for on its own thread, so the window doesn't wait and the
    // finished process is cleaned up.
    std::thread::spawn(move || child.wait());
    Ok(())
}
