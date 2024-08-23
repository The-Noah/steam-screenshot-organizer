use std::{fs, sync::mpsc, thread, time::Duration};

use notify::{Config, RecommendedWatcher, Watcher};

mod steam;
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

      // kill any existing instances of the program

      let exe_name = current_exe.file_name().unwrap().to_string_lossy();
      let system = sysinfo::System::new_all();

      for (pid, process) in system.processes() {
        if process.name().to_string_lossy() != exe_name || pid.as_u32() == std::process::id() {
          continue;
        }

        process.kill();
      }

      if update_handler::update() {
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
        println!("{} {}", env!("CARGO_PKG_NAME"), update_handler::get_current_version());
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
        let latest_version = update_handler::get_latest_version();

        println!("{} {}", env!("CARGO_PKG_NAME"), update_handler::get_current_version());
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
        if let Ok(latest_version) = latest_version {
          println!(
            "Update available: {}",
            if update_handler::is_up_to_date(&update_handler::get_current_version(), &latest_version) {
              "No"
            } else {
              "Yes"
            }
          );
          if !update_handler::is_up_to_date(&update_handler::get_current_version(), &latest_version) {
            println!("Current version: v{}", env!("CARGO_PKG_VERSION"));
            println!("Latest version: {}", latest_version);
          }
        } else {
          print!("Failed to check for updates");
        }
      }
      "run" => run(),
      "watch" => watch(),
      "update" => {
        update_handler::update();
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

#[cfg(target_os = "linux")]
fn hide_console_window() {
  todo!("hide_console_window");
}

#[cfg(target_os = "windows")]
fn add_to_startup() {
  let subkey = r#"Software\Microsoft\Windows\CurrentVersion\Run"#;
  let name = "Steam Screenshot Organizer";

  // check if the program is already in startup

  match win32utils::registry::exists(win32utils::registry::HKEY::CurrentUser, subkey, name) {
    Ok(true) => {
      let path = std::env::current_exe().unwrap();
      let existing_path = win32utils::registry::read_string(win32utils::registry::HKEY::CurrentUser, subkey, name).unwrap_or_default();

      if existing_path == path.to_string_lossy() {
        return;
      }
    }
    Ok(false) => {
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
      eprintln!("Failed to check registry key: {}", error.to_string());
      return;
    }
  }

  let path = std::env::current_exe().unwrap();

  if win32utils::registry::write_string(win32utils::registry::HKEY::CurrentUser, &subkey, &name, path.to_string_lossy()).is_err() {
    eprintln!("Failed to write registry key");
  }
}

#[cfg(target_os = "linux")]
fn add_to_startup() {
  todo!("add_to_startup");
}
