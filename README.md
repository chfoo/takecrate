# Takecrate

Rust library for adding installer functionality to standalone binaries.

This crate enables CLI applications to be distributed as standalone binaries that can install and uninstall themselves.

[![Crates.io Version](https://img.shields.io/crates/v/takecrate)](https://crates.io/crates/takecrate)
[![docs.rs](https://img.shields.io/docsrs/takecrate)](https://docs.rs/takecrate)

## Quick start

```rust
let app_id = AppId::new("com.example.my-app").unwrap();
let manifest = PackageManifest::new(&app_id).with_self_exe().unwrap();

if exe_name.ends_with("_installer") {
    takecrate::install_interactive(&manifest).unwrap();
}
```

## Features

This crate aims to be a safe and easy way for users to use binaries by automating the file copying and search path modification.

Supported OS families: unix (macOS and Linux), windows.

In addition, notable quality of life features include:

* Including files bundled beside the binary.
* Option for installing for the current user or for all users.

## Contributing & support

* [Contributing](https://github.com/chfoo/takecrate/blob/main/.github/CONTRIBUTING.md)
* [Support](https://github.com/chfoo/takecrate/blob/main/.github/SUPPORT.md)

## License

Copyright 2024 Christopher Foo. Licensed under Mozilla Public License 2.0
