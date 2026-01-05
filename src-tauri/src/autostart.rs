use std::path::PathBuf;

/// Create or remove an autostart entry that launches the app at login.
pub fn set_autostart(enable: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if enable {
            enable_windows_autostart()
        } else {
            disable_windows_autostart()
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if enable {
            Err("Autostart is only supported on Windows in this build".to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
fn enable_windows_autostart() -> Result<(), String> {
    let startup_script = startup_script_path()?;
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;

    if let Some(parent) = startup_script.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let exe = exe_path
        .to_str()
        .ok_or_else(|| "Executable path contains invalid characters".to_string())?;

    // Use a small batch script to start the app; quotes handle spaces in paths.
    let contents = format!(
        "@echo off\r\nstart \"\" \"{}\"\r\n",
        exe.replace('\"', "\"\"")
    );

    std::fs::write(&startup_script, contents).map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn disable_windows_autostart() -> Result<(), String> {
    let startup_script = startup_script_path()?;
    if startup_script.exists() {
        std::fs::remove_file(&startup_script).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn startup_script_path() -> Result<PathBuf, String> {
    let mut path = dirs::data_dir().ok_or_else(|| "Cannot find app data directory".to_string())?;
    path.push("Microsoft");
    path.push("Windows");
    path.push("Start Menu");
    path.push("Programs");
    path.push("Startup");
    path.push("ivLyrics-helper-start.bat");
    Ok(path)
}
