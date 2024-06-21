#!/bin/bash
set -e

BASE_PATH="$(dirname "$0")/.."
DOCKERFILE_HASH="$(sha256sum Dockerfile | cut -d' ' -f1)"
DOCKERFILE_IMAGE="docker-registry.intranet.1io.com/build/co:$DOCKERFILE_HASH"

docker build -f Dockerfile -t "$DOCKERFILE_IMAGE" .
docker push "$DOCKERFILE_IMAGE"

echo "variables:" > "$BASE_PATH/ci/build.yml"
echo "  DOCKERFILE_HASH: $DOCKERFILE_HASH" >> "$BASE_PATH/ci/build.yml"
echo "  DOCKERFILE_IMAGE: $DOCKERFILE_IMAGE" >> "$BASE_PATH/ci/build.yml"
