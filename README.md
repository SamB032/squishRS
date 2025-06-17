# Squish

> Compact, compress, and deduplicate files into a single archive format.

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
Clone and Build locally:
``` shell
git clone https://github.com/your-username/squishRS.git
cd squishRS
cargo build --release

```

## 📌 Usage

### Pack
``` shell 
squish pack ./my-folder -o archive.squish
```

### List
``` shell
squish list archive.squish
```
optional simpified format:
``` shell 
squish list archive.squish --simple
```

### Unpack
``` shell
squish unpack archive.squish -o ./output-dir
```

## 📚 Example
``` shell
squish pack ./data -o data.squish
squish list data.squish
squish unpack data.squish -o ./restored

```

## 🛠 Development
Run Tests:
``` shell
cargo test

```

Run Linter (Clippy):
``` shell
cargo clippy --all-targets --all-features -- -D warnings
```

Run Formatter:
``` shell
cargo fmt
```

## 🔬 Internals

- Built on `zstd`, `indicatif`, `prettytable` and `clap`
- Chunk-based deduplication logic
- Archives include a manifest mapping files to their chunks for accurate reconstruction
- Simple `.squish` archive format optimized for speed, space-saving, and portability

## 🙌 Contributions

Contributions, issues, and feature requests are welcome!
Feel free to open an issue or submit a pull request.

## 📄 License
MIT License @ 2025
