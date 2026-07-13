# Changelog

## [Unreleased]

### Changed

- The room core now reduces typed `EventContent` variants instead of matching magic-string notice bodies:
  - `EventContent::Receipt` — `ReceiptKind::Read` advances `Room.read_receipts`, `ReceiptKind::Received` advances `Room.received_receipts` (both monotonic per-sender cursors)
  - `EventContent::Typing` advances `Room.typing`
  - `EventContent::RoomAvatar` applies to `Room.avatar: Option<Avatar>`
  - `EventContent::PinnedEvents` applies a per-event last-write-wins merge (by timestamp) into `Room.pinned_messages`, backed by a new `pinned_at: BTreeMap<String, u64>` guard — replacing the previous wholesale-replace of the pinned list
- **Breaking:** Removed the legacy `__READ_RECEIPT__`, `__RECEIVED_RECEIPT__`, and `__TYPING__` control-notice branches from `reduce_message`; these are now handled exclusively via the typed `EventContent` variants above. `__CHECKLIST_ADD__` and poll notices are unaffected.

## [0.1.0] - 2026-03-31

Initial release.
