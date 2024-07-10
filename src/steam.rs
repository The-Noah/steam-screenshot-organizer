use std::{fs, path, path::Path, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Game {
  #[serde(rename = "appID")]
  pub app_id: u64,
  #[serde(rename = "name")]
  pub name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Games {
  #[serde(rename = "game")]
  pub games: Vec<Game>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename = "gamesList")]
struct GamesList {
  pub games: Games,
}

pub fn get_id() -> Option<u64> {
  let directories = fs::read_dir(r#"C:\Program Files (x86)\Steam\userdata"#).unwrap().collect::<Vec<_>>();

  for directory in directories {
    if directory.is_err() {
      continue;
    }

    let directory = directory.unwrap();

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

pub fn id_to_id3(steam_id: u64) -> String {
  format!("[U:1:{}]", steam_id)
}

pub fn get_online_library(steam_id3: &String) -> Vec<Game> {
  let games = reqwest::blocking::get(format!("https://steamcommunity.com/profiles/{}/games?xml=1", steam_id3))
    .unwrap()
    .text()
    .unwrap();

  let games: GamesList = serde_xml_rs::from_str(&games).unwrap();
  games.games.games
}

pub fn get_screenshots() -> Vec<PathBuf> {
  let mut files = Vec::new();

  for file in fs::read_dir(get_screenshots_directory()).unwrap().collect::<Vec<_>>() {
    if file.is_err() {
      continue;
    }

    let file = file.unwrap();

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
    let app_info = directory.join(format!("appmanifest_{}.acf", app_id));

    if !app_info.exists() {
      continue;
    }

    let app_info = fs::read_to_string(app_info);

    if let Err(error) = app_info {
      eprintln!("Error reading app info: {}", error);
      break;
    }

    let app_info = app_info.unwrap();

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
          return path::absolute(PathBuf::from(line.split('"').nth(3).unwrap())).unwrap();
        }
      }
    }
  }

  #[allow(deprecated)]
  PathBuf::from(std::env::home_dir().unwrap().to_string_lossy().to_string())
    .join("Pictures")
    .join("Steam Screenshots")
}

#[cfg(target_os = "linux")]
pub fn get_screenshots_directory() -> PathBuf {
  #[allow(deprecated)]
  PathBuf::from(std::env::home_dir().unwrap().to_string_lossy().to_string())
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
fn get_app_directories() {
  todo!("get_app_directories");
}
