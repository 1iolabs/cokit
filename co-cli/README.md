# co-cli

## Native core compilation

To support native core ahead of time compilation LLVM is required:

To install LLVM:
```sh
brew install llvm@18
```

To build co-cli with native compilation support:
```sh
cargo build -p co-cli -F llvm
```

Note: By default the build.rs of co-runtime use brew command to add relavant paths:
```
LLVM_SYS_180_PREFIX="$(brew --prefix llvm@18)" LIBRARY_PATH="$(brew --prefix zstd)/lib"
```
