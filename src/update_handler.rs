use std::cmp::Ordering;
use std::io::Read;

use reqwest::{header::USER_AGENT, Method};
use serde::Deserialize;
use tempfile::NamedTempFile;

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

#[derive(Debug)]
#[allow(dead_code)]
enum UpdateError {
  NetworkError(reqwest::Error),
  IoError(std::io::Error),
  ValidationError(String),
  SecurityError(String),
}

impl From<reqwest::Error> for UpdateError {
  fn from(err: reqwest::Error) -> Self {
    UpdateError::NetworkError(err)
  }
}

impl From<std::io::Error> for UpdateError {
  fn from(err: std::io::Error) -> Self {
    UpdateError::IoError(err)
  }
}

const MAX_EXECUTABLE_SIZE: u64 = 10 * 1024 * 1024; // 10MB limit

pub fn get_current_version() -> String {
  format!("v{}", env!("CARGO_PKG_VERSION"))
}

pub fn get_latest_version() -> Result<String, reqwest::Error> {
  let releases = get_releases()?;

  Ok(releases[0].tag_name.clone())
}

fn validate_executable_content(content: &[u8]) -> Result<(), UpdateError> {
  if content.len() > MAX_EXECUTABLE_SIZE as usize {
    return Err(UpdateError::SecurityError(format!(
      "Executable size {} exceeds maximum allowed size {}",
      content.len(),
      MAX_EXECUTABLE_SIZE
    )));
  }

  // Basic PE/ELF header validation
  #[cfg(windows)]
  {
    if content.len() < 64 || &content[0..2] != b"MZ" {
      return Err(UpdateError::SecurityError("Invalid Windows executable format".to_string()));
    }
  }

  #[cfg(unix)]
  {
    if content.len() < 4 || &content[0..4] != b"\x7fELF" {
      return Err(UpdateError::SecurityError("Invalid ELF executable format".to_string()));
    }
  }

  Ok(())
}

fn secure_download(url: &str) -> Result<Vec<u8>, UpdateError> {
  let client = reqwest::blocking::Client::builder().timeout(std::time::Duration::from_secs(300)).build()?;

  let response = client.get(url).header(USER_AGENT, "steam-screenshot-manager").send()?;

  if !response.status().is_success() {
    return Err(UpdateError::NetworkError(response.error_for_status().unwrap_err()));
  }

  let content_length = response.content_length().unwrap_or(0);
  if content_length > MAX_EXECUTABLE_SIZE {
    return Err(UpdateError::SecurityError(format!(
      "Download size {content_length} exceeds maximum allowed size {MAX_EXECUTABLE_SIZE}"
    )));
  }

  let mut content = Vec::new();
  let mut limited_reader = response.take(MAX_EXECUTABLE_SIZE);
  limited_reader.read_to_end(&mut content)?;

  validate_executable_content(&content)?;
  Ok(content)
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

fn atomic_replace_executable(new_content: &[u8]) -> Result<(), UpdateError> {
  use std::{fs, io::Write};

  let current_exe = std::env::current_exe().map_err(UpdateError::IoError)?;

  let exe_dir = current_exe
    .parent()
    .ok_or_else(|| UpdateError::ValidationError("Cannot determine executable directory".to_string()))?;

  // Create temporary file in same directory to ensure atomic move
  let mut temp_file = NamedTempFile::new_in(exe_dir)?;
  temp_file.write_all(new_content)?;

  // Ensure all data is written to disk
  temp_file.flush()?;

  let _temp_path = temp_file.path().to_owned();

  // Create backup of current executable
  let backup_path = current_exe.with_extension("bak");
  if backup_path.exists() {
    fs::remove_file(&backup_path)?;
  }

  // Atomic operations: rename current to backup, then temp to current
  fs::rename(&current_exe, &backup_path)?;

  // Persist the temp file and move it to final location
  match temp_file.persist(&current_exe) {
    Ok(_) => {
      // Success - remove backup
      let _ = fs::remove_file(&backup_path);
      Ok(())
    }
    Err(persist_error) => {
      // Rollback: restore from backup
      let _ = fs::rename(&backup_path, &current_exe);
      Err(UpdateError::IoError(persist_error.error))
    }
  }
}

fn get_latest_version_executable_url() -> Result<String, UpdateError> {
  let releases = get_releases()?;

  if releases.is_empty() {
    return Err(UpdateError::ValidationError("No releases found".to_string()));
  }

  let asset_name = format!("{}{}", env!("CARGO_PKG_NAME"), if cfg!(windows) { ".exe" } else { "" });
  let asset = releases[0]
    .assets
    .iter()
    .find(|asset| asset.name == asset_name)
    .ok_or_else(|| UpdateError::ValidationError(format!("Asset {asset_name} not found in release")))?;

  Ok(asset.browser_download_url.clone())
}

#[cfg(not(debug_assertions))]
pub fn update() -> bool {
  let current_version = get_current_version();

  let latest_version = match get_latest_version() {
    Ok(version) => version,
    Err(e) => {
      eprintln!("Failed to check for updates: {}", e);
      return false;
    }
  };

  if is_up_to_date(&current_version, &latest_version) {
    println!("Already up to date");
    return false;
  }

  #[cfg(target_os = "windows")]
  match win32utils::dialog(
    "Steam Screenshot Organizer",
    format!(
      "An update is available!\n\nCurrent version: {}\nNew version: {}\n\nWould you like to update?",
      current_version, latest_version
    ),
    win32utils::DialogIcon::Question,
    win32utils::DialogButtons::YesNo,
  ) {
    win32utils::DialogResult::Yes => (),
    _ => return false,
  }

  let executable_url = match get_latest_version_executable_url() {
    Ok(url) => url,
    Err(e) => {
      eprintln!("Failed to get download URL: {:?}", e);
      return false;
    }
  };

  let executable_content = match secure_download(&executable_url) {
    Ok(content) => content,
    Err(e) => {
      eprintln!("Failed to download update: {:?}", e);
      return false;
    }
  };

  match atomic_replace_executable(&executable_content) {
    Ok(()) => {
      println!("Updated to version {}", latest_version);
      true
    }
    Err(e) => {
      eprintln!("Failed to install update: {:?}", e);
      false
    }
  }
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
