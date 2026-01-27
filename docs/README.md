# CO Documentation

## Use

### Install

```shell
cargo binstall mdbook@0.4 mdbook-admonish@1.20 mdbook-toc@0.14 mdbook-mermaid@0.15
```

### Run

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
ln -sn ../../target/doc ./src/crate
```
