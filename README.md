<div align="center">
<img width="200" alt="logo" src="https://github.com/user-attachments/assets/bab844a7-ac04-4f72-8838-c543073562ba" />

# rVentoy
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen?style=for-the-badge&logo=githubactions&logoColor=white)](https://github.com/mmdparsa-dev/rVentoy/actions)
[![Rust](https://img.shields.io/badge/Rust-100%25-DEA584?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![WinUI 3](https://img.shields.io/badge/UI-WinUI%203%20%7C%20WASDK-0078D4?style=for-the-badge&logo=windows11&logoColor=white)](https://learn.microsoft.com/en-us/windows/apps/winui/winui3/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011-00ADEF?style=for-the-badge&logo=windows&logoColor=white)](https://microsoft.com/windows)
[![Target](https://img.shields.io/badge/Target-x86__64-informational?style=for-the-badge&logo=intel&logoColor=white)](https://github.com/mmdparsa-dev/rVentoy)
[![Ventoy Core](https://img.shields.io/badge/dynamic/regex?url=https%3A%2F%2Fraw.githubusercontent.com%2Fmmdparsa-dev%2FrVentoy%2Fmain%2Fventoy%2Fversion&search=(.*)&label=Ventoy%20Core&color=5C6BC0&style=for-the-badge&logo=usb&logoColor=white)](https://github.com/ventoy/Ventoy)
[![Reactor](https://img.shields.io/badge/Windows%20Reactor-For%20Rust-7B1FA2?style=for-the-badge&logo=speedtest&logoColor=white)](https://github.com/mmdparsa-dev/rVentoy)
[![Privileges](https://img.shields.io/badge/Privileges-Administrator%20Required-critical?style=for-the-badge&logo=windows-terminal&logoColor=white)](https://github.com/mmdparsa-dev/rVentoy)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-success?style=for-the-badge&logo=gnu&logoColor=white)](LICENSE)
[![GitHub Release](https://img.shields.io/github/v/release/mmdparsa-dev/rVentoy?style=for-the-badge&logo=github&color=2bbc8a)](https://github.com/mmdparsa-dev/rVentoy/releases)
[![Issues](https://img.shields.io/github/issues/mmdparsa-dev/rVentoy?style=for-the-badge&logo=github)](https://github.com/mmdparsa-dev/rVentoy/issues)
[![Stars](https://img.shields.io/github/stars/mmdparsa-dev/rVentoy?style=for-the-badge&logo=github)](https://github.com/mmdparsa-dev/rVentoy/stargazers)

**rVentoy** is a modern and reliable Rust-based rewrite of the Ventoy disk installer with a native WinUI 3 interface.
</div>

---

## ⚡ Overview

[Ventoy](https://github.com/ventoy/Ventoy) revolutionized multiboot USB creation by eliminating the need to reformat drives for every new ISO. **rVentoy** brings that core magic into the modern era:
- Translating legacy C disk-management logic into structured, modern Rust.
- Replacing outdated UI designs with a native, responsive **WinUI 3 (Fluent Design)** interface.
- Delivering robust disk partitioning, VTSI structure handling, and safe byte-level operations.

---

## ✨ Features

- 🦀 **Powered by Rust & Windows Reactor:** Significantly reduces memory-safety risks compared to the original C implementation, wrapping low-level Win32 physical disk I/O with structured abstractions.
- 🎨 **Modern Windows Experience:** Native WinUI 3 interface with Mica material, smooth transitions, and intuitive drive selection.
- 💾 **Precise Disk Layout:** Strict adherence to Ventoy's MBR/GPT partition alignment, EFI partition embedding, and custom magic headers.
- 📦 **Modern Crates:** Backed by high-performance Rust crates (such as `fatfs` and `lzma-rs`) for clean, modular file operations.
- 🛡️ **GPL-3.0 Licensed:** Fully open-source and committed to software freedom.

---

## 🛠️ Getting Started

### Prerequisites

* **Rust toolchain:** Latest stable Rust (`rustup default stable`)
* **Target:** `x86_64-pc-windows-msvc`
* **Windows SDK:** Windows 10/11 SDK installed (for WinUI 3 compilation)
* **Administrator Privileges:** Writing raw sectors to physical disks requires running the executable as Administrator.

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/mmdparsa-dev/rVentoy.git
cd rVentoy

```


2. Compile in release mode:
```bash
cargo build --release

```


3. The generated executable will be available at:
```text
target/release/rventoy.exe
```



---

## ⚠️ Safety Warning

Writing directly to physical disks can cause permanent data loss if the wrong drive is selected. Always verify your target USB drive letter and drive index before proceeding with installation or formatting.

---

## 📜 Attributions & License

This project is a direct Rust port/rewrite of components from the original [Ventoy](https://github.com/ventoy/Ventoy) project by **longpanda**.

* **Original Ventoy Implementation:** Copyright (C) 2020-2026 longpanda `<admin@ventoy.net>`
* **Rust Port & Modern UI:** Copyright (C) 2026 Parsa (`mmdparsa-dev`) and contributors

This project is licensed under the **GNU General Public License v3.0 (GPL-3.0)**.

See the [LICENSE](LICENSE) file for the full license text.
