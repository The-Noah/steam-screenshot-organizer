use std::{fs, sync::mpsc, thread, time::Duration};

use notify::{Config, RecommendedWatcher, Watcher};

mod steam;
#[cfg(not(debug_assertions))]
mod update_handler;

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let args = &args[1..];

  if args.is_empty() {
    if has_console_window() {
      run();
    } else {
      hide_console_window();
      add_to_startup();

      let current_exe = std::env::current_exe().unwrap();

      // kill any existing instances of the program with same path
      let current_exe_path = current_exe.to_string_lossy().to_string();
      let system = sysinfo::System::new_all();

      for (pid, process) in system.processes() {
        // Skip current process
        if pid.as_u32() == std::process::id() {
          continue;
        }

        // Only kill processes with exact same executable path
        if let Some(process_exe) = process.exe() {
          if process_exe.to_string_lossy() == current_exe_path {
            let _ = process.kill(); // Use graceful termination
          }
        }
      }

      #[cfg(not(debug_assertions))]
      let updated = update_handler::update();
      #[cfg(debug_assertions)]
      let updated = false;

      if updated {
        #[cfg(target_os = "windows")]
        win32utils::dialog(
          "Steam Screenshot Organizer",
          "Update successful!\nSteam Screenshot Organizer will now run in the background.",
          win32utils::DialogIcon::Info,
          win32utils::DialogButtons::Ok,
        );

        let args = args.to_vec();
        thread::spawn(move || {
          std::process::Command::new(current_exe).args(args).status().unwrap();
        });

        // ensure the new process has time to start
        thread::sleep(Duration::from_secs(5));
      } else {
        thread::spawn(|| {
          win32utils::shell::tray_icon("Steam Screenshot Manager");
        });

        watch();
      }
    }
  } else {
    match args[0].as_str() {
      "help" | "--help" | "-h" => {
        #[cfg(not(debug_assertions))]
        let version = update_handler::get_current_version();
        #[cfg(debug_assertions)]
        let version = format!("v{}", env!("CARGO_PKG_VERSION"));

        println!("{} {}", env!("CARGO_PKG_NAME"), version);
        println!();
        println!("Usage:");
        println!("  {} [command]", env!("CARGO_PKG_NAME"));
        println!();
        println!("Commands:");
        println!("  help      Display this help message.");
        println!("  info      Display helpful information.");
        println!("  run       Run the program.");
        println!("  watch     Run the program in watch mode.");
        println!("  update    Download any available updates.");
        println!();
        println!("Defaults:");
        println!("  When executed inside a console, the run command is executed.");
        println!("  When executed outside a console, the watch command is executed.");
      }
      "info" => {
        let steam_id = steam::get_id();
        let steam_id3 = steam_id.map(steam::id_to_id3);

        #[cfg(not(debug_assertions))]
        let latest_version = update_handler::get_latest_version();

        #[cfg(not(debug_assertions))]
        let current_version = update_handler::get_current_version();
        #[cfg(debug_assertions)]
        let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));

        println!("{} {}", env!("CARGO_PKG_NAME"), current_version);
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
        #[cfg(not(debug_assertions))]
        if let Ok(latest_version) = latest_version {
          println!(
            "Update available: {}",
            if update_handler::is_up_to_date(&current_version, &latest_version) {
              "No"
            } else {
              "Yes"
            }
          );
          if !update_handler::is_up_to_date(&current_version, &latest_version) {
            println!("Current version: v{}", env!("CARGO_PKG_VERSION"));
            println!("Latest version: {latest_version}");
          }
        } else {
          println!("Failed to check for updates");
        }

        #[cfg(debug_assertions)]
        println!("Update checking disabled in debug builds");
      }
      "run" => run(),
      "watch" => watch(),
      "update" => {
        #[cfg(not(debug_assertions))]
        update_handler::update();
        #[cfg(debug_assertions)]
        println!("Update functionality disabled in debug builds");
      }
      _ => println!("Invalid command."),
    }
  }
}

fn run() {
  let screenshots = steam::get_screenshots();

  println!("Found {} screenshots", screenshots.len());

  let steam_id = steam::get_id();
  let mut online_library = None;

  let mut screenshots_moved = 0;

  for screenshot in &screenshots {
    let game_id = screenshot.file_name().unwrap().to_string_lossy().split('_').next().unwrap().parse::<u64>().unwrap();

    let game_name = if let Some(game_name) = steam::get_app_info(game_id) {
      game_name
    } else {
      if steam_id.is_some() && online_library.is_none() {
        println!("Fetching online library");
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
      if let Err(error) = fs::create_dir(&game_directory) {
        eprintln!("Error creating game directory {}: {}", game_directory.display(), error);
        continue;
      }
    }

    // move screenshot to game directory
    let new_screenshot = game_directory.join(screenshot.file_name().unwrap());
    if let Err(error) = fs::rename(screenshot, new_screenshot) {
      eprintln!("Error moving {}: {}", screenshot.file_name().unwrap().to_string_lossy(), error);
      continue;
    }

    screenshots_moved += 1;

    println!("Moved {} to {}", screenshot.file_name().unwrap().to_string_lossy(), &game_name);
  }

  println!("Moved {}/{} screenshots", screenshots_moved, screenshots.len());
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
      Err(error) => eprintln!("Watch error: {error:?}"),
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

#[cfg(not(target_os = "windows"))]
fn has_console_window() -> bool {
  todo!("has_console_window");
}

#[cfg(target_os = "windows")]
fn hide_console_window() {
  use windows::Win32::{
    Foundation::HANDLE,
    System::Console::{FreeConsole, SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE},
  };

  unsafe {
    SetStdHandle(STD_INPUT_HANDLE, HANDLE(std::ptr::null_mut())).unwrap();
    SetStdHandle(STD_OUTPUT_HANDLE, HANDLE(std::ptr::null_mut())).unwrap();
    SetStdHandle(STD_ERROR_HANDLE, HANDLE(std::ptr::null_mut())).unwrap();
  }

  unsafe { FreeConsole().unwrap() };
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window() {
  todo!("hide_console_window");
}

#[cfg(target_os = "windows")]
fn add_to_startup() {
  let subkey = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
  let name = "Steam Screenshot Organizer";

  // Get current executable path with error handling
  let current_path = match std::env::current_exe() {
    Ok(path) => path,
    Err(e) => {
      eprintln!("Failed to get current executable path: {e}");
      return;
    }
  };

  // Validate that the executable path is reasonable (basic security check)
  let path_str = current_path.to_string_lossy().to_string();
  if path_str.len() > 260 || path_str.contains("..") || !path_str.ends_with(".exe") {
    eprintln!("Invalid executable path for startup registration");
    return;
  }

  // Check if already in startup with correct path
  match win32utils::registry::exists(win32utils::registry::HKEY::CurrentUser, subkey, name) {
    Ok(true) => {
      let existing_path = win32utils::registry::read_string(win32utils::registry::HKEY::CurrentUser, subkey, name).unwrap_or_default();

      if existing_path == path_str {
        return; // Already correctly registered
      }

      // Path differs, update it
      println!("Updating startup path from '{existing_path}' to '{path_str}'");
    }
    Ok(false) => {
      // Not in startup, ask user permission
      match win32utils::dialog(
        "Steam Screenshot Organizer",
        "Would you like to add Steam Screenshot Organizer to startup?",
        win32utils::DialogIcon::Question,
        win32utils::DialogButtons::YesNo,
      ) {
        win32utils::DialogResult::Yes => (),
        _ => return,
      }
    }
    Err(error) => {
      eprintln!("Failed to check startup registry: {}", error.to_string());
      return;
    }
  }

  // Write to registry with error handling
  match win32utils::registry::write_string(win32utils::registry::HKEY::CurrentUser, subkey, name, path_str) {
    Ok(()) => println!("Successfully added to startup"),
    Err(error) => eprintln!("Failed to add to startup registry: {}", error.to_string()),
  }
}

#[cfg(not(target_os = "windows"))]
fn add_to_startup() {
  todo!("add_to_startup");
}
