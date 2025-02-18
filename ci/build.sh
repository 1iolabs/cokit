#!/bin/bash
set -e

BASE_PATH="$(dirname "$0")/.."
DOCKERFILE_HASH="$(sha256sum Dockerfile | cut -d' ' -f1)"
DOCKERFILE_IMAGE="docker-registry.intranet.1io.com/build/co:$DOCKERFILE_HASH"

echo "> $DOCKERFILE_IMAGE"
docker buildx build --push --platform linux/arm64,linux/amd64 -f Dockerfile -t "$DOCKERFILE_IMAGE" .

echo "variables:" > "$BASE_PATH/ci/build.yml"
echo "  DOCKERFILE_HASH: $DOCKERFILE_HASH" >> "$BASE_PATH/ci/build.yml"
echo "  DOCKERFILE_IMAGE: $DOCKERFILE_IMAGE" >> "$BASE_PATH/ci/build.yml"

## Setup buildx:
# brew install colima
# docker buildx create --name multiplatform-builder
# docker buildx use multiplatform-builder
# docker buildx inspect --bootstrap

## Start buildx service:
# brew services start colima
# docker buildx use multiplatform-builder
# docker buildx inspect --bootstrap

## Run buildx service in foreground:
# /opt/homebrew/opt/colima/bin/colima start -f
