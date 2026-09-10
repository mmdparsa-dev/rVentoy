# Contributing to rVentoy

Thank you for your interest in contributing to **rVentoy**! We welcome contributions ranging from low-level disk I/O optimizations to WinUI 3 UX polish.

Because this application writes raw bytes to physical disks, code correctness, safety, and strict alignment with Ventoy standards are critical.

---

## Code of Conduct

Be respectful, constructive, and collaborative. Report bugs transparently and help keep the development environment welcoming to all contributors.

---

## Development Setup

Ensure your local environment meets these requirements:

* **Rust:** Latest stable toolchain (`x86_64-pc-windows-msvc`)
* **C++ / Windows Build Tools:** Visual Studio 2022 (v143 build tools) with C++ desktop workload
* **Windows SDK:** Windows 10 SDK (10.0.19041.0+) or Windows 11 SDK
* **Windows App SDK:** Required for compiling WinUI 3 components

Verify the environment by running:

```bash
cargo check
cargo test

```

---

## Architectural Guidelines

When writing or reviewing code, follow these principles:

* **Safety First (`unsafe` scoping):** Wrap raw Win32 physical disk calls (`CreateFileW`, `DeviceIoControl`, `WriteFile`) inside well-audited, safe Rust abstractions. Document all safety invariants thoroughly with `// SAFETY:` comments.
* **Format Adherence:** Changes affecting MBR/GPT table generation, EFI partition offsets, or VTSI structures must strictly match the upstream Ventoy disk layout specification.
* **Separation of Concerns:** Keep core disk manipulation logic independent of the WinUI 3 presentation layer. UI interactions should dispatch operations through channels or structured async actors.
* **Fluent Design Consistency:** Ensure WinUI 3 controls respect system accent colors, light/dark themes, and high-DPI scaling gracefully.

---

## Workflow & Pull Requests

1. **Fork & Branch:** Create a feature branch off `main` with a descriptive name:
```bash
git checkout -b feat/mbr-alignment-fix

```


2. **Format & Lint:** Ensure all code conforms to standard formatting and passes lint checks:
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings

```


3. **Commit Conventions:** Write clear, concise commit messages following standard conventional commits:
* `feat: add partition type override support`
* `fix: correct cluster offset in FAT32 formatter`
* `docs: update build instructions for MSVC`


4. **Testing Hardware Notice:** Never run physical write tests against primary drives. Use virtual disk files (VHD/VHDX) or dedicated test USB drives when verifying raw write behavior.
5. **Open a PR:** Describe what changed, link relevant issues, and include screenshots or recordings if making visual WinUI modifications.

---

## License

By contributing to **rVentoy**, you agree that your contributions will be licensed under the project's [GNU General Public License v3.0 (GPL-3.0)](https://www.google.com/search?q=LICENSE).
