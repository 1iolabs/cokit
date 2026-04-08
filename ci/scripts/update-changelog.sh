#!/usr/bin/env bash
set -euo pipefail

# updates changelogs for publishable crates using git-cliff
# run this before cargo release, review changes, then commit
#
# usage:
#   ./ci/scripts/update-changelog.sh              # all crates
#   ./ci/scripts/update-changelog.sh co-storage    # single crate

ALL_CRATE_DIRS=(
    co-macros co-primitives co-actor co-storage co-api co-messaging
    co-runtime co-identity co-log co-network co-js co-sdk co-dioxus
    co-bindings co-cli tauri-plugin-co-sdk
    cores/board cores/co cores/file cores/keystore cores/membership
    cores/names cores/rich-text cores/room cores/storage
)

# filter to a single crate if argument provided
if [ $# -gt 0 ]; then
    FILTER="$1"
    CRATE_DIRS=()
    for crate_dir in "${ALL_CRATE_DIRS[@]}"; do
        crate_name=$(grep '^name' "${crate_dir}/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
        if [ "${crate_name}" = "${FILTER}" ] || [ "${crate_dir}" = "${FILTER}" ]; then
            CRATE_DIRS+=("${crate_dir}")
        fi
    done
    if [ ${#CRATE_DIRS[@]} -eq 0 ]; then
        echo "ERROR: crate '${FILTER}' not found"
        exit 1
    fi
else
    CRATE_DIRS=("${ALL_CRATE_DIRS[@]}")
fi

UPDATED=()
SKIPPED=()

for crate_dir in "${CRATE_DIRS[@]}"; do
    crate_name=$(grep '^name' "${crate_dir}/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')
    tag_pattern="${crate_name}-v*"
    changelog="${crate_dir}/CHANGELOG.md"

    # skip if no previous tag exists (initial release)
    if ! git tag -l "${tag_pattern}" | grep -q .; then
        SKIPPED+=("${crate_name} (no previous tag)")
        continue
    fi

    # check if there are unreleased commits for this crate
    latest_tag=$(git tag -l "${tag_pattern}" --sort=-v:refname | head -1)
    commit_count=$(git log "${latest_tag}..HEAD" --oneline -- "${crate_dir}" | wc -l | tr -d ' ')

    if [ "${commit_count}" -eq 0 ]; then
        SKIPPED+=("${crate_name} (no changes since ${latest_tag})")
        continue
    fi

    echo "> updating ${crate_name}: ${commit_count} commit(s) since ${latest_tag}"
    git-cliff \
        --workdir . \
        --config cliff.toml \
        --include-path "${crate_dir}/**" \
        --unreleased \
        --prepend "${changelog}"

    UPDATED+=("${crate_name}")
done

echo ""
if [ ${#UPDATED[@]} -gt 0 ]; then
    echo "Updated: ${UPDATED[*]}"
fi
if [ ${#SKIPPED[@]} -gt 0 ]; then
    echo "Skipped:"
    for entry in "${SKIPPED[@]}"; do
        echo "  - ${entry}"
    done
fi
