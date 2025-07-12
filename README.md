# Squish

> Compact, compress, and deduplicate files into a single archive format.

[![Crates.io](https://img.shields.io/crates/v/squishrs.svg)](https://crates.io/crates/squishrs)
<!--
[![Docs.rs](https://docs.rs/squishrs/badge.svg)](https://docs.rs/squishrs)
-->

---

## 🧭 Table of Contents

- [Features](#-features)
- [Installation](#-installation)
- [Usage](#-usage)
  - [Pack](#pack)
  - [List](#list)
  - [Unpack](#unpack)
- [Example](#-example)
- [Development](#-development)
- [Internals](#-internals)
- [Contributions](#-contributions)
- [License](#-license)

## 🚀 Features

- Pack entire directories into a `.squish` archive
- Automatically deduplicates duplicate files/chunks
- Compact archive format with compression
- List archive contents with summaries
- Unpack files with directory preservation

## 📥 Installation
> **Note:** Ensure you have [Rust and Cargo installed](https://www.rust-lang.org/tools/install).

### 📦 Via Crates
``` shell
cargo install squishrs
```

### 🔧 Clone and Build Locally:
``` shell
git clone https://github.com/SamB032/squishRS.git
cd squishRS
cargo build --release

```

## 📌 Usage

### Pack
``` shell 
squishrs pack ./my-folder -o archive.squish
```

### List
``` shell
squishrs list archive.squish
```

### Unpack
``` shell
squishrs unpack archive.squish -o ./output-dir
```

### Help
``` shell
squishrs --help
```

## 📚 Example
``` shell
squishrs pack ./data -o data.squish
squishrs list data.squish
squishrs unpack data.squish -o ./restored

```

## 🛠 Development
Run commonly used commands from the provided `Makefile`:
```shell
> make help
all             Build the release binary
build           Build binary
run             Run target file with arguments
test            Run tests
fmt             Format code with rustfmt
lint            Run clippy linter
check           Run basic type-checking
clean           Clean build artifacts
install         Install the binary system-wide
help            Show help for each target
```

## 🔬 Internals

- Built on `zstd`, `indicatif`, `prettytable` and `clap`
- Chunk-based deduplication logic
- Archives include a manifest mapping files to their chunks for accurate reconstruction
- Simple `.squish` archive format optimized for speed, space-saving, and portability
- Parallelized file and directory traversal for fast performance on large datasets using `Rayon`

## 🙌 Contributions

Contributions, issues, and feature requests are welcome!
Feel free to open an issue or submit a pull request.

## 📄 License
MIT License @ 2025
