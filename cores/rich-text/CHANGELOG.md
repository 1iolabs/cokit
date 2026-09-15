# Changelog

## [Unreleased]

### Added

- `InsertionPoint::After` (tag `a`) inserts behind the scalar at a position, including a deleted scalar.

### Changed

- `TextModel::insert` anchors non-empty text after the preceding scalar, or before the first scalar at the start, and
  carries the attributes resolved at the cursor as a replace operation.
- Rich-text positions, model indices, and ranges now consistently use UTF-8 byte offsets.
  Edit starts and half-open range ends must be Unicode scalar boundaries.
  An omitted delete or format end affects one scalar.

## [0.1.0] - 2026-03-31

Initial release.
