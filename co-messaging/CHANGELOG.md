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
  - `PublicReceiptContent`, `PrivateReceipt`, `PrivateReceiptContent`
- The same structs now have a `Default` derive so the correct way to initialize them now is either `let mut struct = Struct::default();` and then setting the needed fields directly or using the `new()` functions that all structs have now

### Added

- The structs that didn't already have one, now also have an impl block with a `new()` function to help initialize those structs

## [0.1.0] - 2026-03-31

Initial release.
