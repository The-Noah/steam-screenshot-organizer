use std::{fs, sync::mpsc};

use notify::{Config, RecommendedWatcher, Watcher};

mod steam;

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
        let steam_id = steam::get_id();
        let steam_id3 = steam_id.map(steam::id_to_id3);

        println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!();
        println!("Steam ID: {}", if let Some(steam_id) = steam_id { steam_id.to_string() } else { "Not found".to_string() });
        println!("Steam screenshots directory: {}", steam::get_screenshots_directory().display());
        println!(
          "Online Steam library: {} games found",
          if let Some(steam_id3) = steam_id3 {
            steam::get_online_library(&steam_id3).len()
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
  let screenshots = steam::get_screenshots();

  println!("Found {} screenshots", screenshots.len());

  let steam_id = steam::get_id();
  let mut online_library = None;

  for screenshot in screenshots {
    let game_id = screenshot.file_name().unwrap().to_string_lossy().split('_').next().unwrap().parse::<u64>().unwrap();

    let game_name = if let Some(game_name) = steam::get_app_info(game_id) {
      println!("Game found locally for {}", game_id);
      game_name
    } else {
      if steam_id.is_some() && online_library.is_none() {
        online_library = Some(steam::get_online_library(&steam::id_to_id3(steam_id.unwrap())));
      }

      if let Some(online_library) = &online_library {
        let game = online_library.iter().find(|game| game.app_id == game_id);

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
    let game_directory = steam::get_screenshots_directory().join(&game_name);
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

  watcher.watch(&steam::get_screenshots_directory(), notify::RecursiveMode::NonRecursive).unwrap();

  for event in rx {
    match event {
      Ok(_) => {
        run();
      }
      Err(e) => eprintln!("Watch error: {:?}", e),
    }
  }
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
