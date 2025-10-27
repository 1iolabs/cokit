# CO Documentation

## Run

```shell
mdbook serve -o
```

## Crates

To generate the rustdoc run:

```shell
cargo doc --no-deps
```

To link it to view it using `mdbook serve`:

```shell
cd docs
ln -s ../../target/doc ./src/crate
```
