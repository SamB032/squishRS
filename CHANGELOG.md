# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2025-07-06
### Added
- Switch from `sha256` to `xxhas128` for better performance and compression size
- Add error when incompatiable versions for squish files are detected
- Output squish version in `pack command`

### Fixed
- Better error messages through the use of custom errors

### Changed
- `max-threads` argument applies globally

## [1.0.0] - 2025-06-28
### Added
- Initial stable release of `squishrs`
- Support for `pack`, `unpack`, and `list` subcommands
- Parallel file rebuild using Rayon
- CLI powered by `clap`

### Fixed
- Better error handling for missing chunks during unpacking

### Changed
- Improved progress bar integration
