//! Add installer functionality to standalone binaries.
//!
//! This crate enables CLI applications to be distributed as standalone
//! binaries that can install and uninstall themselves.
//!
//! ## Example
//!
//! ```
//! # use takecrate::manifest::AppId;
//! # use takecrate::inst::PackageManifest;
//! # let exe_name = "";
//! let app_id = AppId::new("com.example.my-app").unwrap();
//! let manifest = PackageManifest::new(&app_id).with_self_exe().unwrap();
//!
//! if exe_name.ends_with("_installer") {
//!     takecrate::install_interactive(&manifest).unwrap();
//! }
//! ```
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
//! On Unix, the user's .profile file might be be corrupted.
//!
//! ### Terminal security
//!
//! If a GUI terminal is launched with administrator-level permissions by the OS,
//! it may be possible for a user to access unauthorized administrator-level
//! resources through the terminal application
//! (e.g., right-clicking the window, selecting Properties, a help link,
//! and launching a web browser that can open and run arbitrary files).
//!
use error::InstallerError;
use inst::{InstallConfig, Installer, PackageManifest};
use manifest::{AppId, DiskManifest};
use path::PathResolver;
use uninst::Uninstaller;

pub mod error;
pub mod inst;
mod locale;
pub mod manifest;
pub mod os;
pub mod path;
mod tui;
pub mod uninst;

/// Starts the installer with a interactive interface.
///
/// A terminal user interface (TUI) will guide the user with installation
/// options and perform the installation.
///
/// If the user cancels the guide, the error kind [`InterruptedByUser`](crate::error::InstallerErrorKind::InterruptedByUser)
/// will be returned. If an error occurs, an appropriate error kind will be
/// returned.
pub fn install_interactive(manifest: &PackageManifest) -> Result<(), InstallerError> {
    let mut installer = Installer::new(manifest);
    installer.run_interactive()
}

/// Installs the binary to the device with the given configuration.
///
/// This function is intended for "quiet" installs where the installation
/// occurs automatically, such as, a shell script.
pub fn install(manifest: &PackageManifest, config: &InstallConfig) -> Result<(), InstallerError> {
    let mut installer = Installer::new(manifest);
    installer.run(config)
}

/// Starts the uninstaller with a interactive interface.
///
/// A terminal user interface (TUI) will prompt for a confirmation before
/// the uninstallation proceeds.
///
/// If there is both a User and System installation, the uninstaller will
/// uninstall the User version.
pub fn uninstall_interactive(app_id: &AppId) -> Result<(), InstallerError> {
    let mut uninstaller = Uninstaller::new(app_id);
    uninstaller.run_interactive()
}

/// Uninstalls the binary from the device with the given application UUID.
///
/// This function is intended for "quiet" uninstalls where the uninstallation
/// occurs automatically, such as, a shell script.
///
/// If there is both a User and System installation, the uninstaller will
/// uninstall the User version.
pub fn uninstall(app_id: &AppId) -> Result<(), InstallerError> {
    let mut uninstaller = Uninstaller::new(app_id);
    uninstaller.run()
}

/// Returns the disk manifest when the binary is installed.
///
/// If there is both a User and System installation, the User version will
/// be returned.
pub fn manifest(app_id: &AppId) -> Result<DiskManifest, InstallerError> {
    let exe_path = std::env::current_exe()?;
    crate::manifest::discover_manifest(&exe_path, app_id)
}

/// Returns the path resolver suitable for ths binary.
///
/// Use this path resolver when you need to access data files installed along
/// with the binary.
///
/// If there is both a User and System installation, the User version will
/// be returned.
pub fn path_resolver(app_id: &AppId) -> Result<PathResolver, InstallerError> {
    let manifest = manifest(app_id)?;
    let resolver = PathResolver::new(app_id.plain_id(), &manifest.app_path_prefix)?;
    Ok(resolver)
}
