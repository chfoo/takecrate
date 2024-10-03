//! OS specific functionalities.

pub use std::env::current_exe;
use std::{
    ffi::{OsStr, OsString},
    fs::File,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::{InstallerError, InstallerErrorKind};

#[cfg(unix)]
pub(crate) mod unix;
#[cfg(windows)]
pub(crate) mod windows;

/// OS specific error wrapper.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OsError {
    /// Standard IO error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Error type provided by the Windows crates by Microsoft.
    #[cfg(windows)]
    #[error(transparent)]
    Windows(#[from] windows_result::Error),

    /// Any other error.
    #[error("{0}")]
    Other(&'static str),
}

impl From<OsError> for InstallerError {
    fn from(value: OsError) -> Self {
        InstallerError::new(InstallerErrorKind::Io).with_source(value)
    }
}

/// Specifies who can use the binary on a machine.
///
/// See also [`crate::path::AppPathPrefix`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessScope {
    /// For the current user only.
    User,
    /// For all users.
    System,
}

impl Default for AccessScope {
    fn default() -> Self {
        Self::User
    }
}

/// Information returned by [`file_checksum`].
#[derive(Debug, Clone, PartialEq)]
pub struct FileChecksum {
    /// CRC32C checksum of the file.
    pub crc32c: u32,
    /// Size of the file.
    pub len: u64,
}

/// Computes a checksum for a file.
pub fn file_checksum<P: AsRef<Path>>(path: P) -> std::io::Result<FileChecksum> {
    let path = path.as_ref();
    tracing::trace!(?path, "file checksum");
    let len = path.metadata()?.len();

    let mut file = File::open(path)?;
    let mut compute = crc32c::Crc32cWriter::new(std::io::empty());

    std::io::copy(&mut file, &mut compute)?;

    Ok(FileChecksum {
        crc32c: compute.crc32c(),
        len,
    })
}

/// Returns the filename portion of the current execuable's path.
///
/// See also [`std::env::current_exe()`].
pub fn current_exe_name() -> std::io::Result<OsString> {
    tracing::trace!("current_exe_name");
    std::env::current_exe()?
        .file_name()
        .map(|name| name.to_os_string())
        .ok_or_else(|| std::io::Error::other(InstallerErrorKind::UnknownExecutablePath))
}

/// Returns the directory portion of the current executable's path.
///
/// See also [`std::env::current_exe()`].
pub fn current_exe_dir() -> std::io::Result<PathBuf> {
    tracing::trace!("current_exe_dir");
    let mut path = std::env::current_exe()?;
    path.pop();

    Ok(path)
}

pub(crate) fn env_var<A: AsRef<OsStr>>(key: A) -> Result<OsString, InstallerError> {
    tracing::trace!(key = ?key.as_ref(), "env_var");
    std::env::var_os(key.as_ref())
        .ok_or_else(|| InstallerErrorKind::InvalidEnvironmentVariable.into())
}
