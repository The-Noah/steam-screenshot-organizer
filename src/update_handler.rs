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

#[cfg(not(debug_assertions))]
pub fn update() -> bool {
  use std::{
    fs::{self, File},
    io::Write,
  };

  fn get_latest_version_executable_url() -> Result<String, reqwest::Error> {
    let releases = get_releases()?;

    let asset = releases[0].assets.iter().find(|asset| asset.name == "steam-screenshot-organizer.exe").unwrap();

    Ok(asset.browser_download_url.clone())
  }

  let current_version = get_current_version();
  let latest_version = get_latest_version().unwrap();

  if current_version == latest_version {
    println!("Already up to date");
    return false;
  }

  let executable_url = get_latest_version_executable_url().unwrap();
  let executable = reqwest::blocking::get(&executable_url).unwrap().bytes().unwrap();

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
