use std::{path::PathBuf, sync::mpsc};

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

  if args.len() == 0 || args[0] == "run" {
    run();
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
      }
      "debug" => {
        let steam_id = get_steam_id();
        let steam_id3 = if let Some(steam_id) = steam_id { Some(steam_id_to_id3(steam_id)) } else { None };

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
          "Steam library: {} games found",
          if let Some(steam_id3) = steam_id3 {
            get_steam_library(&steam_id3).games.games.len()
          } else {
            0
          }
        );
      }
      "watch" => {
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
      _ => println!("Invalid command."),
    }
  }
}

fn run() {
  let steam_id = get_steam_id();

  if steam_id.is_none() {
    println!("Steam ID not found.");
    return;
  }

  let library = get_steam_library(&steam_id_to_id3(steam_id.unwrap()));

  let screenshots = get_screenshots();

  for screenshot in screenshots {
    let game_id = screenshot.file_name().unwrap().to_string_lossy().split("_").next().unwrap().parse::<u64>().unwrap();

    for game in library.games.games.iter() {
      if game.app_id == game_id {
        // ensure game directory exists
        let game_directory = get_screenshots_directory().join(&game.name);
        if !game_directory.exists() {
          std::fs::create_dir(&game_directory).unwrap();
        }

        // move screenshot to game directory
        let new_screenshot = game_directory.join(screenshot.file_name().unwrap());
        std::fs::rename(&screenshot, &new_screenshot).unwrap();

        println!("Moved {} to {}", screenshot.file_name().unwrap().to_string_lossy(), game.name);

        break;
      }
    }
  }
}

fn get_steam_id() -> Option<u64> {
  let directories = std::fs::read_dir(r#"C:\Program Files (x86)\Steam\userdata"#).unwrap().collect::<Vec<_>>();

  for directory in directories {
    if !directory.is_ok() {
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
  let games = reqwest::blocking::get(&format!("https://steamcommunity.com/profiles/{}/games?xml=1", steam_id3))
    .unwrap()
    .text()
    .unwrap();

  serde_xml_rs::from_str(&games).unwrap()
}

fn get_screenshots() -> Vec<PathBuf> {
  let mut files = Vec::new();

  for file in std::fs::read_dir(get_screenshots_directory()).unwrap().collect::<Vec<_>>() {
    if !file.is_ok() {
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

fn get_screenshots_directory() -> PathBuf {
  PathBuf::from(std::env::home_dir().unwrap().to_string_lossy().to_string())
    .join("Pictures")
    .join("Steam Screenshots")
}
