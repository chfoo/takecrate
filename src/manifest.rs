//! Describing the contents an installed package.
//!
//! In order to ensure user and system files are not overwritten or removed
//! inadvertently, the crate makes use of file lists stored on disk.
//!
//! ## Default file locations
//!
//! Disk manifests will be located in:
//!
//! * `$HOME/.config/io.crates.takecrate/`
//! * `/var/local/lib/io.crates.takecrate/`
//! * `%LocalAppData%/io.crates.takecrate/`
//! * `%ProgramData%/io.crates.takecrate/`
//!
//! Disk manifest files are named `takecrate-manifest__[app-id].ron`
//! where `[app-id`] is the namespaced ID format.
//!

pub use self::discovery::*;
pub use self::disk::*;
pub use self::id::*;

mod discovery;
mod disk;
mod id;
