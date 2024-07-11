use std::cmp::Ordering;

use reqwest::{header::USER_AGENT, Method};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Release {
  tag_name: String,
  assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Asset {
  name: String,
  browser_download_url: String,
}

pub fn get_current_version() -> String {
  format!("v{}", env!("CARGO_PKG_VERSION"))
}

pub fn get_latest_version() -> Result<String, reqwest::Error> {
  let releases = get_releases()?;

  Ok(releases[0].tag_name.clone())
}

pub fn is_up_to_date(current: &str, new: &str) -> bool {
  let current = current.trim_start_matches('v');
  let new = new.trim_start_matches('v');

  let current: Vec<u32> = current.split('.').map(|s| s.parse().unwrap()).collect();
  let new: Vec<u32> = new.split('.').map(|s| s.parse().unwrap()).collect();

  for (current, new) in current.iter().zip(new.iter()) {
    match current.cmp(new) {
      Ordering::Less => return false,
      Ordering::Greater => return true,
      Ordering::Equal => (),
    }
  }

  true
}

#[cfg(not(debug_assertions))]
pub fn update() -> bool {
  use std::{
    fs::{self, File},
    io::Write,
  };

  fn get_latest_version_executable_url() -> Result<String, reqwest::Error> {
    let releases = get_releases()?;

    let asset_name = format!("{}{}", env!("CARGO_PKG_NAME"), if cfg!(windows) { ".exe" } else { "" });
    let asset = releases[0].assets.iter().find(|asset| asset.name == asset_name).unwrap();

    Ok(asset.browser_download_url.clone())
  }

  let current_version = get_current_version();
  let latest_version = get_latest_version().unwrap();

  if is_up_to_date(current_version, latest_version) {
    println!("Already up to date");
    return false;
  }

  #[cfg(target_os = "windows")]
  unsafe {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONQUESTION, MB_YESNO};

    let title: Vec<u16> = "Steam Screenshot Organizer\0".encode_utf16().collect();
    let text: Vec<u16> = format!(
      "An update is available!\n\nCurrent version: {}\nNew version: {}\n\nWould you like to update?\0",
      current_version, latest_version
    )
    .encode_utf16()
    .collect();

    let answer = MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_YESNO | MB_ICONQUESTION);

    match answer {
      IDYES => (),
      _ => return false,
    }
  }

  let executable_url = get_latest_version_executable_url().unwrap();
  let executable = reqwest::blocking::get(executable_url).unwrap().bytes().unwrap();

  let old_exe_path = std::env::current_dir()
    .unwrap()
    .join(format!("{}.bak", std::env::current_exe().unwrap().file_stem().unwrap().to_str().unwrap()));

  if old_exe_path.exists() {
    fs::remove_file(&old_exe_path).expect("Failed to delete old version");
  }

  fs::rename(std::env::current_exe().unwrap(), &old_exe_path).expect("Failed to rename current version");

  let mut file = File::create(std::env::current_exe().unwrap()).unwrap();
  file.write_all(&executable).unwrap();

  if old_exe_path.exists() {
    fs::remove_file(&old_exe_path).expect("Failed to delete old version");
  }

  println!("Updated to version {}", latest_version);

  true
}

#[cfg(debug_assertions)]
pub fn update() -> bool {
  false
}

fn get_releases() -> Result<Vec<Release>, reqwest::Error> {
  let client = reqwest::blocking::Client::new();
  client
    .request(Method::GET, "https://api.github.com/repos/The-Noah/steam-screenshot-organizer/releases")
    .header(USER_AGENT, "steam-screenshot-manager")
    .send()?
    .json::<Vec<Release>>()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_up_to_date() {
    assert!(is_up_to_date("v1.0.0", "v1.0.0"));

    assert!(is_up_to_date("v1.0.1", "v1.0.0"));
    assert!(is_up_to_date("v1.1.0", "v1.0.0"));

    assert!(!is_up_to_date("v1.0.0", "v1.0.1"));
    assert!(!is_up_to_date("v1.0.0", "v1.1.0"));

    assert!(!is_up_to_date("v1.0.0", "v2.0.0"));
    assert!(is_up_to_date("v2.0.0", "v1.0.0"));
  }
}
