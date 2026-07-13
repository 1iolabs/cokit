# Changelog

## [Unreleased]

### Changed

- **Breaking:** These structs now have the `#[non_exhaustive]` annotation and can therefore no longer be initialized with `Struct { ... }`:
  - `MatrixEvent`
  - `ImageInfo`, `ThumbnailInfo`, `AudioInfo`, `VideoInfo`, `FileInfo`, `LocationInfo`
  - `RoomNameContent`, `RoomTopicContent`, `RoomAvatarContent`, `PinnedEventsContent`
  - `TypingContent`, `PresenceContent`
  - `Mentions`, `TextContent`, `NoticeContent`, `ImageContent`, `AudioContent`, `VideoContent`, `FileContent`, `LocationContent`
  - `PollStartContent`, `PollCreationInfo`, `PollAnswer`, `PollResponseContent`, `PollEndContent`
  - `PostUserStoryContent`, `ViewUserStoryContent`, `UpdateProfileContent`
  - `ReactionContent`, `RelatesTo`, `ReplyContent`, `RedactionContent`
  - `SessionDescription`, `ICECandidate`, `CallInviteContent`, `AnswerCallContent`, `CallCandidatesContent`, `SelectCallAnswerContent`, `CallNegotiationContent`, `RejectCallContent`, `HangupCallContent`
  - `ReceiptContent`, `PrivateReceipt`, `PrivateReceiptContent`
- **Breaking:** `PublicReceiptContent` is replaced by `ReceiptContent { kind: ReceiptKind, up_to, thread_id }` (its old `read` field is renamed `up_to`), with a new `ReceiptKind` enum (`Read`, `Received`) so the type now covers both read and received (delivered) receipts.
- **Breaking:** `RoomAvatarContent` is reshaped to carry `avatar: Option<Avatar>` (was `{ file: Option<Cid>, info: ImageInfo }`), constructed via the new `RoomAvatarContent::new(Avatar)` / `RoomAvatarContent::remove()`. New `Avatar` enum: `Image { cid, info }` or `Emoji(String)`.
- **Breaking:** `PinnedEventsContent` is reshaped from `{ pinned: Vec<String> }` (whole-list) to a per-event toggle `{ event_id: String, pinned: bool }`, constructed via `PinnedEventsContent::new(event_id, pinned)`. Pins now merge per event instead of wholesale-replacing the list.
- These structs now have a `Default` derive:
  - `ImageInfo`, `ThumbnailInfo`, `AudioInfo`, `VideoInfo`, `FileInfo`, `LocationInfo`
  - `PinnedEventsContent`
  - `TypingContent`
  - `Mentions`
  - `PollEndContent`
  - `ReactionContent`, `RelatesTo`

### Added

- The structs that didn't already have one, now also have an impl block with a `new()` function to help initialize those structs

## [0.1.0] - 2026-03-31

Initial release.
