[English](README.md)

# arch-installer

[![Лицензия: GPL-3.0](https://img.shields.io/badge/Лицензия-GPL--3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

`arch-installer` — простой менеджер пакетов для установки Arch Linux пакетов (`.pkg.tar.zst`) на любом Linux. Поддерживает установку, удаление, переустановку, список пакетов и системную информацию.

## Возможности
- Установка/удаление/переустановка пакетов.
- Проверка ELF, иконок, `.desktop`-файлов.
- Логи в `~/.local/share/arch-installer/`.
- Поддержка `sudo`/`doas`, зависимостей.

## Установка
```bash
Скачай последний бинарник (arch-installer-release)
chmod +x arch-installer-release
sudo cp arch-installer-release /usr/bin/arch-installer
```

## Сборка из исходников
```bash
git clone https://github.com/OverLessArtem/arch-installer.git
cargo build --release
sudo cp target/release/arch-installer /usr/bin/arch-installer
```

## Команды
- **Установить**:
  ```bash
  sudo arch-installer install cmatrix.pkg.tar.zst [--prefix=/path]
  ```
- **Удалить**:
  ```bash
  sudo arch-installer uninstall cmatrix.pkg.tar.zst [--prefix=/path]
  ```
- **Переустановить**:
  ```bash
  sudo arch-installer reinstall cmatrix.pkg.tar.zst [--prefix=/path]
  ```
- **Список пакетов**:
  ```bash
  arch-installer list
  ```
  Вывод: `1` (для `cmatrix`).
- **Системная информация**:
  ```bash
  arch-installer info
  ```
  Вывод:
  ```
  OS: Arch Linux
  Kernel: 6.16.0
  Shell: bash
  DE: KDE
  Packages: pacman 1234, arch-installer 1
  ```

## Лицензия
[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0).
