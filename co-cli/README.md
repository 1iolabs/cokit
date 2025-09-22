# co-cli

## Native core compilation

To support native core ahead of time compilation LLVM is required:

To install LLVM:
```sh
brew install llvm@15
```

To build co-cli with native compilation support:
```sh
LLVM_SYS_150_PREFIX="$(brew --prefix llvm@15)" LIBRARY_PATH="$(brew --prefix zstd)/lib" cargo build -p co-cli -F llvm
```
