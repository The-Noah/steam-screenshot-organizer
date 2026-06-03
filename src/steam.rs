use std::{fs, path::PathBuf};

#[cfg(target_os = "windows")]
use std::path::{self, Path};

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppDetailsData {
  pub name: String,
}

#[derive(Debug, Deserialize)]
struct AppDetails {
  pub success: bool,
  pub data: Option<AppDetailsData>,
}

#[cfg(target_os = "windows")]
fn steam_root() -> Option<PathBuf> {
  Some(PathBuf::from(r#"C:\Program Files (x86)\Steam"#))
}

#[cfg(target_os = "linux")]
fn steam_root() -> Option<PathBuf> {
  #[allow(deprecated)]
  let home = std::env::home_dir()?;

  [
    home.join(".steam").join("steam"),
    home.join(".local").join("share").join("Steam"),
    home.join(".var").join("app").join("com.valvesoftware.Steam").join(".local").join("share").join("Steam"),
  ]
  .into_iter()
  .find(|path| path.exists())
}

pub fn get_id() -> Option<u64> {
  let userdata = steam_root()?.join("userdata");

  let directories = fs::read_dir(userdata).ok()?;

  for directory in directories.flatten() {
    if !directory.path().is_dir() {
      continue;
    }

    if let Ok(id) = directory.file_name().to_string_lossy().parse::<u64>() {
      if id == 0 {
        continue;
      }

      return Some(id);
    }
  }

  None
}

/// Fetch a game's name from the Steam Web API by app ID.
pub fn get_online_app_info(app_id: u64) -> Option<String> {
  let response = reqwest::blocking::get(format!("https://store.steampowered.com/api/appdetails?appids={app_id}")).ok()?;

  let mut details: HashMap<String, AppDetails> = response.json().ok()?;
  let details = details.remove(&app_id.to_string())?;

  if !details.success {
    return None;
  }

  details.data.map(|data| data.name)
}

pub fn get_screenshots() -> Vec<PathBuf> {
  let mut files = Vec::new();

  let Ok(entries) = fs::read_dir(get_screenshots_directory()) else {
    return files;
  };

  for file in entries.flatten() {
    if !file.path().is_file() {
      continue;
    }

    files.push(file.path());
  }

  files
}

pub fn get_app_info(app_id: u64) -> Option<String> {
  let directories = get_app_directories();

  for directory in directories {
    let app_info = directory.join(format!("appmanifest_{app_id}.acf"));

    if !app_info.exists() {
      continue;
    }

    let app_info = match fs::read_to_string(app_info) {
      Ok(app_info) => app_info,
      Err(error) => {
        eprintln!("Error reading app info: {error}");
        break;
      }
    };

    let app_info = app_info.split('\n').collect::<Vec<_>>();

    for line in app_info {
      let line = line.trim();

      if line.starts_with("\"name\"") {
        let name = line.split('"').collect::<Vec<_>>()[3];

        return Some(name.to_string());
      }
    }
  }

  None
}

#[cfg(target_os = "windows")]
pub fn get_screenshots_directory() -> PathBuf {
  let steam_id = get_id();

  if let Some(steam_id) = steam_id {
    let steam_config_path = PathBuf::new()
      .join(r#"C:\Program Files (x86)\Steam\userdata"#)
      .join(steam_id.to_string())
      .join("config")
      .join("localconfig.vdf");
    let steam_config = std::fs::read_to_string(steam_config_path);

    if let Ok(steam_config) = steam_config {
      for line in steam_config.lines() {
        let line = line.trim();

        if line.starts_with("\"InGameOverlayScreenshotSaveUncompressedPath\"") {
          if let Some(path) = line.split('"').nth(3) {
            if let Ok(absolute) = path::absolute(PathBuf::from(path)) {
              return absolute;
            }
          }
        }
      }
    }
  }

  #[allow(deprecated)]
  std::env::home_dir()
    .unwrap_or_default()
    .join("Pictures")
    .join("Steam Screenshots")
}

#[cfg(target_os = "linux")]
pub fn get_screenshots_directory() -> PathBuf {
  if let (Some(steam_id), Some(root)) = (get_id(), steam_root()) {
    let steam_config_path = root.join("userdata").join(steam_id.to_string()).join("config").join("localconfig.vdf");

    if let Ok(steam_config) = std::fs::read_to_string(steam_config_path) {
      for line in steam_config.lines() {
        let line = line.trim();

        if line.starts_with("\"InGameOverlayScreenshotSaveUncompressedPath\"") {
          if let Some(path) = line.split('"').nth(3) {
            if let Ok(absolute) = std::path::absolute(PathBuf::from(path)) {
              return absolute;
            }
          }
        }
      }
    }
  }

  #[allow(deprecated)]
  std::env::home_dir()
    .unwrap_or_default()
    .join("Pictures")
    .join("Steam Screenshots")
}

#[cfg(target_os = "windows")]
fn get_app_directories() -> Vec<PathBuf> {
  use windows::Win32::Storage::FileSystem::GetLogicalDrives;

  let drives = unsafe { GetLogicalDrives() };
  let mut drive_id = 0b0000001;

  let mut directories = vec![PathBuf::new().join("C:\\").join("Program Files (x86)").join("Steam").join("steamapps")];
  let mut drive_letters = vec![];

  for i in 0..26 {
    let drive_letter = char::from(b'A' + i);

    if drives & drive_id != 0 && drive_letter != 'C' {
      let drive_path = PathBuf::new().join(drive_letter.to_string() + ":\\").join("SteamLibrary").join("steamapps");

      if Path::exists(&drive_path) {
        directories.push(drive_path);
      }

      drive_letters.push(drive_letter);
    }

    drive_id <<= 1;
  }

  directories
}

#[cfg(target_os = "linux")]
fn get_app_directories() -> Vec<PathBuf> {
  let mut directories = Vec::new();

  if let Some(root) = steam_root() {
    let steamapps = root.join("steamapps");

    // additional library folders are listed in libraryfolders.vdf
    if let Ok(library_folders) = fs::read_to_string(steamapps.join("libraryfolders.vdf")) {
      for line in library_folders.lines() {
        let line = line.trim();

        if line.starts_with("\"path\"") {
          if let Some(path) = line.split('"').nth(3) {
            let library = PathBuf::from(path).join("steamapps");

            if library != steamapps && library.exists() {
              directories.push(library);
            }
          }
        }
      }
    }

    directories.push(steamapps);
  }

  directories
}
