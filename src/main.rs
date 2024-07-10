use std::{fs, path::PathBuf, sync::mpsc};

use notify::{Config, RecommendedWatcher, Watcher};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Game {
  #[serde(rename = "appID")]
  app_id: u64,
  #[serde(rename = "name")]
  name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Games {
  #[serde(rename = "game")]
  games: Vec<Game>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename = "gamesList")]
struct GamesList {
  games: Games,
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let args = &args[1..];

  if args.is_empty() {
    if has_console_window() {
      run();
    } else {
      hide_console_window();
      watch();
    }
  } else {
    match args[0].as_str() {
      "help" | "--help" | "-h" => {
        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!();
        println!("Usage:");
        println!("  {} [command]", env!("CARGO_PKG_NAME"));
        println!();
        println!("Commands:");
        println!("  help     Display this help message.");
        println!("  debug    Display debug information.");
        println!("  run      Run the program.");
        println!("  watch    Run the program in watch mode.");
        println!();
        println!("Defaults:");
        println!("  When executed inside a console, the run command is executed.");
        println!("  When executed outside a console, the watch command is executed.");
      }
      "debug" => {
        let steam_id = get_steam_id();
        let steam_id3 = steam_id.map(steam_id_to_id3);

        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!();
        println!(
          "Steam ID: {}",
          if let Some(steam_id) = get_steam_id() {
            steam_id.to_string()
          } else {
            "Not found".to_string()
          }
        );
        println!("Steam screenshots directory: {}", get_screenshots_directory().display());
        println!(
          "Online Steam library: {} games found",
          if let Some(steam_id3) = steam_id3 {
            get_steam_library(&steam_id3).games.games.len()
          } else {
            0
          }
        );
      }
      "run" => run(),
      "watch" => watch(),
      _ => println!("Invalid command."),
    }
  }
}

fn run() {
  let screenshots = get_screenshots();

  println!("Found {} screenshots", screenshots.len());

  let steam_id = get_steam_id();
  let mut online_library = None;

  for screenshot in screenshots {
    let game_id = screenshot.file_name().unwrap().to_string_lossy().split('_').next().unwrap().parse::<u64>().unwrap();

    let game_name = if let Some(game_name) = get_steam_app_info(game_id) {
      println!("Game found locally for {}", game_id);
      game_name
    } else {
      if steam_id.is_some() && online_library.is_none() {
        online_library = Some(get_steam_library(&steam_id_to_id3(steam_id.unwrap())));
      }

      if let Some(online_library) = &online_library {
        let game = online_library.games.games.iter().find(|game| game.app_id == game_id);

        if let Some(game) = game {
          game.name.clone()
        } else {
          continue;
        }
      } else {
        continue;
      }
    };

    // ensure game directory exists
    let game_directory = get_screenshots_directory().join(&game_name);
    if !game_directory.exists() {
      fs::create_dir(&game_directory).unwrap();
    }

    // move screenshot to game directory
    let new_screenshot = game_directory.join(screenshot.file_name().unwrap());
    fs::rename(&screenshot, new_screenshot).unwrap();

    println!("Moved {} to {}", screenshot.file_name().unwrap().to_string_lossy(), &game_name);
  }

  println!("Done");
}

fn watch() {
  run();

  let (tx, rx) = mpsc::channel();

  let mut watcher = RecommendedWatcher::new(tx, Config::default()).unwrap();

  watcher.watch(&get_screenshots_directory(), notify::RecursiveMode::NonRecursive).unwrap();

  for event in rx {
    match event {
      Ok(_) => {
        run();
      }
      Err(e) => eprintln!("Watch error: {:?}", e),
    }
  }
}

fn get_steam_id() -> Option<u64> {
  let directories = std::fs::read_dir(r#"C:\Program Files (x86)\Steam\userdata"#).unwrap().collect::<Vec<_>>();

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

fn steam_id_to_id3(steam_id: u64) -> String {
  format!("[U:1:{}]", steam_id)
}

fn get_steam_library(steam_id3: &String) -> GamesList {
  let games = reqwest::blocking::get(format!("https://steamcommunity.com/profiles/{}/games?xml=1", steam_id3))
    .unwrap()
    .text()
    .unwrap();

  serde_xml_rs::from_str(&games).unwrap()
}

fn get_screenshots() -> Vec<PathBuf> {
  let mut files = Vec::new();

  for file in std::fs::read_dir(get_screenshots_directory()).unwrap().collect::<Vec<_>>() {
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

#[cfg(target_os = "windows")]
fn get_screenshots_directory() -> PathBuf {
  let steam_id = get_steam_id();

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
          return std::path::absolute(PathBuf::from(line.split('"').nth(3).unwrap())).unwrap();
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
fn get_screenshots_directory() -> PathBuf {
  #[allow(deprecated)]
  PathBuf::from(std::env::home_dir().unwrap().to_string_lossy().to_string())
    .join("Pictures")
    .join("Steam Screenshots")
}

#[cfg(target_os = "windows")]
fn has_console_window() -> bool {
  use windows::Win32::{System::Console::GetConsoleWindow, UI::WindowsAndMessaging::GetWindowThreadProcessId};

  let console = unsafe { GetConsoleWindow() };

  if console.is_invalid() {
    return false;
  }

  let mut console_pid = 0;
  unsafe { GetWindowThreadProcessId(console, Some(&mut console_pid)) };

  console_pid != std::process::id()
}

#[cfg(target_os = "linux")]
fn has_console_window() -> bool {
  todo!("has_console_window");
}

#[cfg(target_os = "windows")]
fn hide_console_window() {
  use windows::Win32::System::Console::FreeConsole;

  unsafe { FreeConsole().unwrap() };
}

#[cfg(target_os = "linux")]
fn hide_console_window() {
  todo!("hide_console_window");
}

#[cfg(target_os = "windows")]
fn get_steam_app_directories() -> Vec<PathBuf> {
  use std::path::Path;

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
fn get_steam_app_directories() {
  todo!("get_steam_app_directories");
}

fn get_steam_app_info(app_id: u64) -> Option<String> {
  let directories = get_steam_app_directories();

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
