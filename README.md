# Takecrate

Rust library for adding installer functionality to standalone binaries.

This crate enables CLI applications to be distributed as standalone binaries that can install and uninstall themselves.

*Note: Work in progress.*

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

* Supported OS families: unix, windows
* Saves checksums to manifest files and uses them before overwriting or deleting files.
* Installs for the current user or all users.
* Modifies search path (PATH).

## Contributing & support

* [Contributing](https://github.com/chfoo/takecrate/blob/main/.github/CONTRIBUTING.md)
* [Support](https://github.com/chfoo/takecrate/blob/main/.github/SUPPORT.md)

## License

Copyright 2024 Christopher Foo. Licensed under Mozilla Public License 2.0
