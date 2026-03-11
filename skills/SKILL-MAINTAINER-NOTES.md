# COkit Skill - Maintainer Notes

## Version
- **v1.0.0** - Initial release
- **Date**: 2026-02-19
- **Based on**: All documentation in `docs/src/` + public pages (1io.com/en/COkit, 1io.com/en/manifest)

## What Changed
- Initial creation of the `cokit` skill package
- 11 reference files covering all major topics
- SKILL.md with mental model, lookup index, and common pitfalls

## User-Invocable Setting

**Recommendation: `user-invocable: false` (background-only) is the best default.**

Rationale:
- This is a reference/knowledge skill, not an action skill. Users do not need to manually
  invoke it; it should auto-load when COkit-related topics are discussed.
- Background-only reduces noise in the skill list for users who browse available skills.
- If debugging is needed (e.g., verifying the skill loaded), admins can temporarily
  set `user-invocable: true`.

Note: If the Claude Organization Skills spec does not support `user-invocable` in
frontmatter, this is handled at the provisioning level when uploading.

## How to Upload / Provision

1. Navigate to your Claude Organization admin panel.
2. Go to Skills management (or Custom Skills section).
3. Click "Upload Skill" or "Add Custom Skill."
4. Upload the `cokit.zip` file.
5. The system will validate the SKILL.md frontmatter and folder structure.
6. Review the parsed name (`cokit`) and description.
7. Configure visibility: recommend making it available to all org members who work
   with COkit/co-sdk.
8. If the platform supports it, set to background-only (non-user-invocable).
9. Save and activate.

## Trigger Quality

The description was tested against 25 example prompts. Key findings:
- 17/25 prompts correctly trigger (true positives for COkit-specific queries)
- 3/25 correctly do not trigger (true negatives for unrelated queries)
- 5/25 are ambiguous edge cases (generic terms like DID, libp2p, CRDT, Dioxus)

To reduce false positives, the description was refined to:
- Require COkit-adjacent terms alongside generic terms (DID, libp2p, etc.)
- Add specific package names as triggers (co-dioxus, tauri-plugin-co-sdk, co-js)
- Remove bare generic terms from the auto-load trigger list

## Maintenance

When updating COkit documentation:
1. Review the reference files for accuracy against new docs.
2. Update the Source Map sections at the bottom of each reference file.
3. Mark new features as "available" and remove "coming soon" labels as appropriate.
4. Keep SKILL.md under 500 lines; move details to reference files.
5. Re-run the trigger test set if the description changes.
6. Re-zip and re-upload.

## Known TBDs / Single-Source Items

- Cryptography audit status: documented as "not yet audited" (single-source: security.md)
- License publication status: legal notice marks AGPLv3 as "non-operative until official
  publication" (single-source: legal-notice.md)
- Manual consensus (issue #87): planned, not implemented
- Shared/quorum consensus (issue #88): planned, not implemented
- BLE networking (issue #79): coming soon
- WebRTC/WebSocket (issue #89): coming soon
- Wi-Fi Direct (issue #90): coming soon
- Swift bindings (issue #95): coming soon
- Android bindings (issue #96): coming soon
- HTTP networking mode (issue #78): coming soon
