# <img height="24" src="logo.png" /> Steam Screenshot Organizer

This is a simple script that organizes your Steam screenshots into folders based on the game they were taken in.

## Usage

Your screenshot folder is automatically detected from Steam, which you can change by going to `Steam` -> `Settings` -> `In-Game` -> `Screenshot Folder`.

1. Download the [latest release](https://github.com/The-Noah/steam-screenshot-organizer/releases/latest/download/steam-screenshot-organizer.exe)
2. Place the executable in any directory
3. Run the executable. It will run in the background and organize your screenshots. It will also organize new screenshots as you take them.
4. After running the executable you will be given the option to enable automatically running at startup. If you do not enable this, you must rerun the executable whenever you reboot.

Currently only supports Windows. Linux and MacOS will eventually be supported.

## Building

If you would like to build the script yourself, you can do so by following these steps:

0. Install the [Rust toolchain](https://www.rust-lang.org/tools/install)

1. Clone the repository

```bash
git clone https://github.com/The-Noah/steam-screenshot-organizer.git
cd steam-screenshot-organizer
```

2. Build the script

```bash
cargo build --release
```

The executable will be located at `target/release/steam-screenshot-organizer.exe`.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
