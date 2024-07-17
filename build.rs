#[cfg(target_os = "windows")]
pub fn add_resource(name: String) {
  let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR env var");

  if !std::process::Command::new("rc.exe")
    .args(["/fo", &format!("{}/{}.lib", &out_dir, &name), "/I", &out_dir, &(name.to_owned() + &String::from(".rc"))])
    .status()
    .expect("Could not find rc.exe")
    .success()
  {
    panic!("Failed to build resource file");
  }

  println!("cargo:rustc-link-search=native={}", &out_dir);
  println!("cargo:rustc-link-lib=dylib={}", &name);
}

#[cfg(target_os = "windows")]
fn main() {
  add_resource(String::from("resources"));
}

#[cfg(not(target_os = "windows"))]
fn main() {}
