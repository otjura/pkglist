# pkglist

A stylish Rust CLI & TUI tool for Fedora/RHEL/dnf5-based systems to explore installed packages and analyze exactly how much space they and their dependencies consume.

```text
 pkglist | Packages: 3705 | Total Installed Size: 23.85 GB
╭ Search (Press [f] to search) ────────────────────────────────────────────────╮
│Type here to filter packages...                                               │
╰──────────────────────────────────────────────────────────────────────────────╯
╭ Packages (3705) ─────────────────────────────╮╭ Package Details ─────────────╮
│  Package   Type    Size    Transitiv Saved ▼ ││Package: oidn-libs            │
│» oidn-libs rpm     212.10 M 3.11 GB   2.67 GB ││Version: 2.4.0-2.fc44 (x86_64)│
│  rocm-hip  rpm     27.00 MB 2.90 GB   2.46 GB ││Type:    rpm                  │
│  hipcc     rpm     633.62 K 2.73 GB   2.30 GB ││                              │
│  rocm-devi rpm     3.25 MB  2.73 GB   2.30 GB ││Installed Size: 212.10 MB     │
│  rocm-llvm rpm     1.92 GB  2.32 GB   2.00 GB ││Total Deps Size: 3.11 GB      │
╰──────────────────────────────────────────────╯╰──────────────────────────────╯
 [q] Quit | [f] Search | [1-5] Sort Columns | [WASD/Arrows] Scroll / Switch Panels
```

---

## Key Features

### 💻 Interactive TUI Mode
* **Side-by-Side Interface**: Browse packages in a table on the left, with full detailed metadata (description, version, architecture, requirements) updating instantly on the right.
* **Unified WASD & Arrow Controls**: Use classic arrow keys or `WASD` to scroll lists and switch visual panel focus.
* **Fast Search**: Press `f` to search/filter packages in real time as you type. Press `Esc` to instantly clear the search, or `Enter`/`Tab` to apply it.
* **Interactive Dependency Navigation**: Tab into the details card to browse through sorted direct dependencies and direct dependents using arrow/WASD keys. Pressing `Enter` jumps instantly to that package in the main table.

### 🛠 Powerful CLI Mode
* `packages list`: Lists all packages in a beautifully aligned terminal table.
  * Supports sorting by Name, Type, Size, Transitive Size, and Exclusive Size.
  * Outputs in multiple formats: Pretty Table, CSV, TSV, or JSON.
  * Filter results with `--search <query>` and limit rows with `--limit <n>`.
* `packages info <package>`: Dumps a highly structured, colorized text profile of a single package, including sorted requirements and dependents.

### 📐 Disk Space Analysis
* **Installed Size**: The raw space the package takes on disk.
* **Transitive Size (Total dependencies size)**: The total size of the package plus the recursive closure of all its dependencies (everything needed to run it).
* **Saved Space (Exclusive size)**: Calculated using a reference counting garbage collection simulation. It represents the *exact* amount of disk space that will be reclaimed if you uninstall this package (i.e. the package itself plus any of its dependencies that are not required by any other remaining package on your system). Handles dependency cycles and shared dependencies correctly.

---

## TUI Key Bindings

| Key | Action |
| :--- | :--- |
| **`f`** | Open search field |
| **`Esc`** | Exit search & clear search field (or clear filter when not typing) |
| **`Enter`** / **`Tab`** | Close search field (keeping the search query active) |
| **`Tab`** | Switch focus between Packages table and Package Details panel |
| **`Left`** / **`a`** / **`A`** | Focus the Packages table panel |
| **`Right`** / **`d`** / **`D`** | Focus the Package Details panel |
| **`Up`** / **`w`** / **`W`** | Scroll up in the active panel (Table rows or Details lists) |
| **`Down`** / **`s`** / **`S`** | Scroll down in the active panel (Table rows or Details lists) |
| **`PageUp`** / **`PageDown`** | Page scroll up/down in the active panel |
| **`Home`** / **`End`** | Jump to the beginning or end of the active panel |
| **`1`** | Sort packages by Name |
| **`2`** | Sort packages by Type |
| **`3`** | Sort packages by Installed Size |
| **`4`** | Sort packages by Transitive Size (Total dependencies size) |
| **`5`** | Sort packages by Saved Space (Exclusive size) |
| **`Enter`** (on Details) | Jump to the selected dependency or dependent in the Packages table |
| **`q`** / **`Ctrl+C`** | Quit application |

---

## Compilation and Installation

### Prerequisites
* **Rust**: Ensure you have the Rust toolchain installed. If not, get it via [rustup.rs](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
* **DNF5**: The system must have `dnf5` package manager available in the PATH.

### 1. Compile from Source
Clone this repository, navigate to the directory, and compile in release mode:
```bash
cargo build --release
```
The compiled binary will be placed at `./target/release/packages`.

### 2. Install to Local Path
To install the binary to cargo's default binary directory (usually `~/.cargo/bin/`, which should be in your `PATH`):
```bash
cargo install --path .
```

Alternatively, you can manually copy or link the binary to a folder in your path:
```bash
# Copy to user local binaries directory
cp target/release/packages ~/.local/bin/pkglist

# Or symlink it to any other location
sudo ln -sf "$(pwd)/target/release/packages" /usr/local/bin/pkglist
```

---

## CLI Usage Examples

```bash
# Start the interactive TUI
pkglist

# List top 10 packages taking up the most exclusive space (saved if removed)
pkglist list --limit 10 --sort exclusive

# Search for installed python packages and output in JSON format
pkglist list --search python --format json

# Get detailed size analysis for the dnf5 package
pkglist info dnf5
```

---

## 🗺️ Roadmap & Contributions

While `pkglist` was initially designed for `dnf5`-based systems, the core graph and size analysis engines are highly modular. We would love community contributions to expand it to other package managers!

### Backend Abstraction Wishes
We plan to introduce a `PackageManagerBackend` trait:
```rust
pub trait PackageManagerBackend {
    fn load_packages(&self) -> Result<Vec<RawPackage>, String>;
}
```

This will make it straightforward to add support for other ecosystems:
* 📦 **Pacman (Arch Linux)**: Interfacing with `pacman -Q` and virtual provides.
* 📦 **APT/Dpkg (Debian/Ubuntu)**: Interfacing with `dpkg-query` and mapping package constraints.
* 📦 **Apk (Alpine Linux)**: A lightweight backend for auditing Alpine/Docker containers.
* 📦 **Homebrew (macOS)**: Querying installed cellars, sizes, and dependency receipts.
* 📦 **Nix / Guix**: Visualizing store path closure sizes and declarative profiles.

If you are interested in implementing any of these backends or optimizing the graph calculation pipelines, pull requests are warmly welcome!
