//! Add installer functionality to standalone binaries.
//!
//! This crate enables CLI applications to be distributed as standalone
//! binaries that can install and uninstall themselves.
//!
//! ## Overview
//!
//! To get started:
//!
//! 1. Create a unique ID for your application using [`AppId`].
//! 2. Create the listing of input files using [`PackageManifest`].
//! 3. Based on context, run [`install_interactive()`] or [`uninstall_interactive`], or continue normally in your binary.
//! 4. If you need a included data file in a installation, use [`manifest()`] to get a [`DiskManifest`].
//!
//! ## Example
//!
//! ```
//! # use std::error::Error;
//! # use takecrate::manifest::AppId;
//! # use takecrate::inst::PackageManifest;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # let exe_name = "";
//! let app_id = AppId::new("com.example.my-app")?;
//! let manifest = PackageManifest::new(&app_id).with_self_exe()?;
//!
//! if exe_name.ends_with("_installer") {
//!     takecrate::install_interactive(&manifest)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Further reading
//!
//! [More information](crate::lib_doc)

use error::InstallerError;
use inst::{InstallConfig, Installer, PackageManifest};
use manifest::{AppId, DiskManifest};
use uninst::Uninstaller;

pub mod lib_doc;

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
