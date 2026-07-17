# Changelog

## [Unreleased]

### Added

- Added atomic tag-selected task enqueue/replacement and lock-fenced completion actions.

### Changed

- Changed locked task moves used as claims so stale source, task, and lock preconditions are successful no-ops.
- Made duplicate list creation idempotent so concurrent queue bootstrap converges without replacing existing list state.

## [0.1.0] - 2026-03-31

Initial release.
