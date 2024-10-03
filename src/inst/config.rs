use std::path::PathBuf;

use crate::{error::InstallerError, os::AccessScope, path::AppPathPrefix};

/// Parameters that control how the binary is installed.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct InstallConfig {
    /// Access scope.
    pub access_scope: AccessScope,
    /// Where the files will be installed.
    pub destination: AppPathPrefix,
    /// Where the files are coming from.
    pub source_dir: PathBuf,
    /// Whether to modify the search path (PATH).
    ///
    /// On Windows, this will modify the environment variable and App Paths.
    ///
    /// On Unix with user scope, this will modify the user's `.profile` if
    /// the file does not already modify the PATH.
    /// On macOS, the SHELL variable is used to determine whether to write to
    /// `.zprofile` or `.bash_profile` first.
    /// For system scope, it's not supported.
    pub modify_os_search_path: bool,
}

impl InstallConfig {
    /// Create a new config suitable for a User install.
    pub fn new() -> Result<Self, InstallerError> {
        Ok(Self {
            access_scope: Default::default(),
            destination: Default::default(),
            source_dir: crate::os::current_exe_dir()?,
            modify_os_search_path: true,
        })
    }
}
