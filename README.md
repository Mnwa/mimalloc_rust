# Mimalloc3 Rust

[![Latest Version]][crates.io] [![Documentation]][docs.rs]

A drop-in global allocator wrapper around the [mimalloc](https://github.com/microsoft/mimalloc) allocator.
Mimalloc is a general purpose, performance oriented allocator built by Microsoft.

> [!NOTE]
> `mimalloc3` is the modern Rust 2024 edition package that tracks current stable
> mimalloc releases. It currently uses mimalloc `v3.5.1` by default and provides
> mimalloc `v2.5.1` through the `v2` feature. Automated update checks keep both
> release lines current.

## Usage

```rust
use mimalloc3::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

## Requirements

A __C__ compiler is required for building [mimalloc](https://github.com/microsoft/mimalloc) with cargo.

## Usage with secure mode

Using secure mode adds guard pages,
randomized allocation, encrypted free lists, etc. The performance penalty is usually
around 10% according to [mimalloc](https://github.com/microsoft/mimalloc)
own benchmarks.

To enable secure mode, put in `Cargo.toml`:

```ini
[dependencies]
mimalloc3 = { version = "*", features = ["secure"] }
```

## Usage with v2

By default this library uses mimalloc `v3`.
To use MiMalloc `v2`, write in `Cargo.toml`:

```ini
[dependencies]
mimalloc3 = { version = "*", features = ["v2"] }
```

[crates.io]: https://crates.io/crates/mimalloc3
[Latest Version]: https://img.shields.io/crates/v/mimalloc3.svg
[Documentation]: https://docs.rs/mimalloc3/badge.svg
[docs.rs]: https://docs.rs/mimalloc3
