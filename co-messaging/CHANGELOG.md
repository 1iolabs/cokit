# Changelog

## [Unreleased]

### Changed

- **Breaking:** Structs now have the `#[non_exhaustive]` annotation and can therefore no longer be initialized with `Struct { ... }`
- Structs now have a `Default` derive so the correct way to initialize them now is either `let mut struct = Struct::default();` and then setting the needed fields directly or using the `new()` functions that all structs have now

### Added

- The structs that didn't already have one, now also have an impl block with a `new()` function to help initialize those structs

## [0.1.0] - 2026-03-31

Initial release.
