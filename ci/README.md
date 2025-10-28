# CI Build

## Docker

This creates an image tagged with the sha256 of the Dockerfile which can then used for the build.
So everytime the Dockerfile gets changed we need to run the build.sh.

### Build

Setup docker for multiplatform build (macOS):
- Active containerd in docker desktop general settings.

Build:
```shell
./build.sh
```
