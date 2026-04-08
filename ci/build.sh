#!/bin/bash
set -e

BASE_PATH="$(dirname "$(dirname "$(readlink -f "$0")")")"
DOCKERFILE_HASH="$(sha256sum "$BASE_PATH/ci/Dockerfile" | cut -d' ' -f1)"
DOCKERFILE_IMAGE="docker-registry.intranet.1io.com/build/co:$DOCKERFILE_HASH"

echo "> $DOCKERFILE_IMAGE"
cd "$BASE_PATH/ci"
docker buildx build --push --provenance false --platform linux/arm64,linux/amd64 -f Dockerfile -t "$DOCKERFILE_IMAGE" .

echo "variables:" > "$BASE_PATH/ci/build.yml"
echo "  DOCKERFILE_HASH: $DOCKERFILE_HASH" >> "$BASE_PATH/ci/build.yml"
echo "  DOCKERFILE_IMAGE: $DOCKERFILE_IMAGE" >> "$BASE_PATH/ci/build.yml"
