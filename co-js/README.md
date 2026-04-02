# co-js

COKIT primitives for the browser using WebAssembly.

## Run tests

`wasm-pack test --node`

## Build

`npm run build` builds the wasm and bundles an npm package under `./pkg`. It uses wasm-pack plugin to call `wasm-pack build`.
The command uses the crate information from the `Cargo.toml` to gegnerate the npm package.
The scope has to be specified via command line arg `--scope` which can be defined in the webpack config wasm pack plugin.

## Publish

Call `wasm-pack publish` to directly publish the package under `./pkg` to an npm registry.
You can also generate a tarball with `wasm-pack pack`.
