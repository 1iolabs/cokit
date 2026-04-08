# Releasing COKIT

Releases follow a "developer prepares, CI publishes" model. The developer
bumps versions, updates changelogs, commits, tags, and pushes locally.
CI triggers on the tag push and publishes to crates.io and npm.

## Release Types

COKIT has two types of releases:

**Crate releases** publish individual crates to crates.io. Each crate
maintains its own independent semantic version (e.g., `co-storage` 0.1.0 →
0.1.1). Use this for bug fixes, new features, or breaking changes in
specific crates. Tags: `co-storage-v0.1.1`.

**Project milestones** mark significant roadmap achievements that span many
crates. They do not bump any crate versions — they record a tested, coherent
snapshot of which crate versions are known-compatible. Tags: `v0.2.0`.

## Prerequisites

```bash
cargo binstall cargo-release git-cliff
```

## Single Crate Release

```bash
# 1. update changelog
./ci/scripts/update-changelog.sh co-storage

# 2. review and commit
git diff
git add -A && git commit -m "docs: update changelog for co-storage"

# 3. dry-run (verify-changelog hook checks entry exists)
cargo release patch --package co-storage

# 4. release (bumps version, commits, tags, pushes)
cargo release patch --package co-storage --execute --no-confirm

# 5. verify (after CI publishes)
cargo search co-storage
```

## Workspace Release

```bash
# 1. update all changelogs
./ci/scripts/update-changelog.sh
git add -A && git commit -m "docs: update changelogs"

# 2. dry-run
cargo release patch --workspace --unpublished

# 3. release
cargo release patch --workspace --unpublished --execute --no-confirm
```

## Version Levels

```bash
cargo release patch    # 0.1.0 → 0.1.1
cargo release minor    # 0.1.0 → 0.2.0
cargo release major    # 0.1.0 → 1.0.0
cargo release alpha    # 0.1.0 → 0.1.1-alpha.1
cargo release beta     # 0.1.0 → 0.1.1-beta.1
cargo release rc       # 0.1.0 → 0.1.1-rc.1
cargo release release  # 0.1.1-rc.1 → 0.1.1
```

## Project Milestone

Milestones mark roadmap achievements. They do not bump crate versions — they
record a snapshot of which crate versions are known-compatible.

```bash
# 1. ensure all relevant crate releases are published
# 2. tag
git tag -a v0.2.0 -m "COKIT 0.2.0: <description>"
git push origin v0.2.0

# 3. update root CHANGELOG.md with the compatibility table
```

## npm Packages

npm packages (`@1io/co-js`, `@1io/tauri-plugin-co-sdk`) are published by
CI alongside the Rust crates when a tag is pushed. To publish manually:

```bash
(cd co-js && wasm-pack build --target web && cd pkg && npm publish --access public)
(cd tauri-plugin-co-sdk && npm run build && npm publish --access public)
```

## What Happens on Tag Push

When a version tag (e.g., `co-sdk-v0.2.0`) is pushed:

1. CI checks out the tagged commit
2. Runs `cargo publish` for the tagged crate(s)
3. Builds and publishes npm packages
4. For `co-cli-v*` tags: builds prebuilt binaries for all targets

## Troubleshooting

**`cargo release` complains about uncommitted changes:**
Commit or stash your changes first. The release must start from a clean tree.

**`verify-changelog.sh` fails:**
Run `./ci/scripts/update-changelog.sh <crate>` and commit the result.

**CI publish fails with "crate already exists":**
The version was already published. This is not an error — the tag and version
bump are already in git.

**Rate limit on crates.io:**
Wait and retry. `cargo publish` will skip already-published versions.
