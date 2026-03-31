# co-runtime
COKIT WebAssembly Runtime to execute Cores (CO State Reducers).

## LLVM Backend

To use the `llvm` backend feature LLVM needs to be installed to your system.

MacOS:
```shell
brew install llvm@21
echo "Add environment to `.cargo/config.toml` file:"
echo "[env]"
echo "LLVM_SYS_211_PREFIX = \"$("brew" "--prefix" "llvm@21")\""
```
