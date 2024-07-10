# Steam Screenshot Organizer

This is a simple script that organizes your Steam screenshots into folders based on the game they were taken in.

## Usage

Your screenshot folder is automatically detected from Steam, which you can change by going to `Steam` -> `Settings` -> `In-Game` -> `Screenshot Folder`.

1. Download the [latest release](https://github.com/The-Noah/steam-screenshot-organizer/releases/latest/download/steam-screenshot-organizer.exe)
2. Place the executable in any directory
3. Run the executable

The script will automatically find your Steam screenshot directory and organize your screenshots into folders based on the game they were taken in.

Currently, the script only supports Windows (even though there is a Linux binary). Work is being done to support Linux and eventually MacOS.

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
