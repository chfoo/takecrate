//! Documentation further reading.
//!
//! ## Supported OS families
//!
//! * unix
//! * windows
//!
//! ## Included features
//!
//! * Additional files (see later section).
//! * Installation for the current user or for all users.
//! * Modification of the search path (PATH).
//! * Persisting a manifest to record changes to the system.
//! * File checksum comparison for safe file overwrites and removals.
//!
//! For details on the search path, see [`InstallConfig`](crate::InstallConfig).
//!
//! ## Additional files
//!
//! Sometimes, you may need to include additional files beside the binary.
//! This crate does not implement self-extracting archives. However, you
//! can still bundle your files into a zip or tar.gz file and instruct the
//! user to extract them.
//!
//! See [`PackageManifest`](crate::PackageManifest) for details on how to
//! configure the installer to recognize files beside your binary.
//!
//! ## Installer principles
//!
//! The crate tries to be as safe as possible, but complete safety and
//! security promises is not guaranteed.
//!
//! ### Files
//!
//! Before overwriting or removing a file, a checksum is compared
//! to one stored in a manifest.
//! If they don't match, the installation is stopped.
//!
//! ### Search paths
//!
//! When the search path (PATH) is being modified, it's possible that a
//! TOCTOU race condition may cause unwanted behavior if there's another
//! installer operating. On Windows, some paths might not be preserved.
//! On Unix, the user's .profile file might be corrupted.
//!
//! ### Terminal security
//!
//! If a GUI terminal is launched with administrator-level permissions by the OS,
//! it may be possible for a user to access unauthorized
//! resources through the terminal application
//! (e.g., right-clicking the window, selecting Properties, a help link,
//! and launching a web browser that can open and run arbitrary files).
//!
//! ### Localized language support
//!
//! The crate is internationalized using [Fluent](https://projectfluent.org/)
//! and will automatically pick a language for the current locale.
//! Translation files are embedded into the binary.
//! Use feature `i18n-static` and `i18n-custom` to customize this behavior.
//!
//! The translation files are located in the `locales` directory of
//! this crate's source code. If you want to contribute a localization,
//! please see the contributing note in the source repository.
//!
//! ## Cargo feature list
//!
//! * `default`: `ui`, `i18n`, `i18n-static`
//! * `ui`: Enables the terminal user interface (TUI).
//! * `i18n`: Enables internationalization and localization (globalization) support.
//! * `i18n-static`: Enables builtin language translation files.
//! * `i18n-custom`: Enables support for custom language translations.
//! * `ui-theme`: Enables Cursive themes API which exposes "unstable" dependency types.
