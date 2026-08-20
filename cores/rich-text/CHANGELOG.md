# Changelog

## [Unreleased]

### Changed

- Rich-text positions, model indices, and ranges now consistently use UTF-8 byte offsets.
  Edit starts and half-open range ends must be Unicode scalar boundaries.
  An omitted delete or format end affects one scalar.

## [0.1.0] - 2026-03-31

Initial release.
