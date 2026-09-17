# Changelog

## [Unreleased]

### Added

- `InsertionPoint::After` (tag `a`) inserts behind the scalar at a position, including a deleted scalar.
- `TextModel::new` builds a model over a stored state.

### Changed

- `TextModel::insert` anchors text after the preceding scalar.
- Insert actions with empty text are now rejected by the reducer.
- Rich-text positions, model indices, and ranges now consistently use UTF-8 byte offsets.
  Edit starts and half-open range ends must be Unicode scalar boundaries.
  An omitted delete or format end affects one scalar.

## [0.1.0] - 2026-03-31

Initial release.
