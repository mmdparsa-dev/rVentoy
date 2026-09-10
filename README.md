<div align="center">
<img width="200" alt="logo" src="https://github.com/user-attachments/assets/bab844a7-ac04-4f72-8838-c543073562ba" />

# rVentoy
![WinUI 3](https://img.shields.io/badge/UI-WinUI%203-blue?style=flat)
![Rust](https://img.shields.io/badge/Rust-100%25-orange?style=flat)
![Reactor](https://img.shields.io/badge/Windows%20Reactor-for%20Rust-purple?style=flat)
![License](https://img.shields.io/badge/License-GPL-green?style=flat)
</div>

**rVentoy** is a modern and reliable Rust-based rewrite of the Ventoy disk installer with a native WinUI 3 interface.

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
