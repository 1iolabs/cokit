# CI Build

## Docker

This creates an image tagged with the sha256 of the Dockerfile which can then used for the build.
So everytime the Dockerfile gets changed we need to run the build.sh.

### Build

Setup docker for multiplatform build (macOS):
```shell
brew install colima
docker buildx create --name multiplatform-builder
docker buildx use multiplatform-builder
docker buildx inspect --bootstrap
```

Or to use after restart:
```shell
"$HOMEBREW_PREFIX/opt/colima/bin/colima" start
docker buildx use multiplatform-builder
docker buildx inspect --bootstrap
```

Build:
```shell
./build.sh
```
