[Русский](README-RU.md)

# arch-installer

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

`arch-installer` is a simple package manager for installing Arch Linux packages (`.pkg.tar.zst`) on any Linux distribution. Supports installation, uninstallation, reinstallation, package listing, and system info display.

## Features
- Install/uninstall/reinstall packages.
- Validates ELF binaries, icons, `.desktop` files.
- Logs in `~/.local/share/arch-installer/`.
- Supports `sudo`/`doas`, dependencies.

## Installation
```bash
Download latest binary (arch-installer-release)
chmod +x arch-installer-release
sudo cp arch-installer-release /usr/bin/arch-installer
```

## Building from source
```bash
git clone https://github.com/OverLessArtem/arch-installer.git
cargo build --release
sudo cp target/release/arch-installer /usr/bin/arch-installer
```

## Commands
- **Install**:
  ```bash
  sudo arch-installer install cmatrix.pkg.tar.zst [--prefix=/path]
  ```
- **Uninstall**:
  ```bash
  sudo arch-installer uninstall cmatrix.pkg.tar.zst [--prefix=/path]
  ```
- **Reinstall**:
  ```bash
  sudo arch-installer reinstall cmatrix.pkg.tar.zst [--prefix=/path]
  ```
- **List packages**:
  ```bash
  arch-installer list
  ```
  Output: `1` (for `cmatrix`).
- **System info**:
  ```bash
  arch-installer info
  ```
  Output:
  ```
  OS: Arch Linux
  Kernel: 6.16.0
  Shell: bash
  DE: KDE
  Packages: pacman 1234, arch-installer 1
  ```

## License
[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0).
