//! Describing where files are located.
//!
//! This module contains filesystem path abstractions and methods for
//! translating them to real filesystem paths.
//!
//! ## Default file locations
//!
//! For binaries, they will be located in:
//!
//! * `$HOME/.local/bin/`
//! * `/usr/local/bin/`
//! * `%LocalAppData%/Programs/[app-id]/bin/`
//! * `%ProgramFiles%/[app-id]/bin/`
//!
//! `[app-id]` is the plain ID format.
//!
//! For any data files:
//!
//! * `$HOME/.local/share/[app-id]/`
//! * `/usr/local/share/[app-id]/`
//! * `%LocalAppData%/Programs/[app-id]/bin/`
//! * `%ProgramFiles%/[app-id]/bin/`
//!
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::{
    error::{InstallerError, InstallerErrorKind},
    os::AccessScope,
};

// For notes on OS specific paths, please see the submodules of the
// `crate::os` module

/// Specifies abstractly where the files are installed on the machine.
///
/// See also [`AccessScope`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppPathPrefix {
    /// In the current user's account.
    User,
    /// In the system directories accessible for all users.
    System,
    /// In a single directory.
    SingleDir(PathBuf),
    /// With a custom prefix instead of `/usr/local`.
    CustomUnix(PathBuf),
}

impl Default for AppPathPrefix {
    fn default() -> Self {
        Self::User
    }
}

impl From<AccessScope> for AppPathPrefix {
    fn from(value: AccessScope) -> Self {
        match value {
            AccessScope::User => Self::User,
            AccessScope::System => Self::System,
        }
    }
}

#[derive(Debug, Clone)]
enum ResolvedPrefix {
    SingleDir(PathBuf),
    Unix(PathBuf),
}

impl Default for ResolvedPrefix {
    fn default() -> Self {
        Self::SingleDir(PathBuf::default())
    }
}

/// Resolves path abstractions into real file paths.
#[derive(Debug, Clone, Default)]
pub struct PathResolver {
    app_id: String,
    prefix: ResolvedPrefix,
    env_map: Option<HashMap<OsString, OsString>>,
}

impl PathResolver {
    /// Create a new path resolver.
    ///
    /// In order to match an existing installation, the exact arguments
    /// need to match when provided during installation.
    pub fn new<I: AsRef<str>>(app_id: I, prefix: &AppPathPrefix) -> Result<Self, InstallerError> {
        let app_id = app_id.as_ref();

        Self::new_impl(app_id, prefix, None)
    }

    fn new_impl(
        app_id: &str,
        prefix: &AppPathPrefix,
        env_map: Option<HashMap<OsString, OsString>>,
    ) -> Result<Self, InstallerError> {
        let mut self_ = Self {
            prefix: ResolvedPrefix::default(),
            app_id: app_id.to_string(),
            env_map,
        };
        self_.prefix = self_.resolve_prefix(prefix)?;

        Ok(self_)
    }

    fn resolve_prefix(&mut self, prefix: &AppPathPrefix) -> Result<ResolvedPrefix, InstallerError> {
        match prefix {
            AppPathPrefix::User => self.resolve_user_prefix(),
            AppPathPrefix::System => self.resolve_system_prefix(),
            AppPathPrefix::SingleDir(path) => Ok(ResolvedPrefix::SingleDir(path.to_path_buf())),
            AppPathPrefix::CustomUnix(path) => Ok(ResolvedPrefix::Unix(path.to_path_buf())),
        }
    }

    fn resolve_user_prefix(&mut self) -> Result<ResolvedPrefix, InstallerError> {
        match std::env::consts::FAMILY {
            "windows" => {
                let dir = self.get_env_var("LOCALAPPDATA")?;
                let mut dir = PathBuf::from(dir);
                dir.push("Programs");
                dir.push(&self.app_id);

                Ok(ResolvedPrefix::SingleDir(dir))
            }
            "unix" => {
                let dir = self.get_env_var("HOME")?;
                let mut dir = PathBuf::from(dir);
                dir.push(".local");

                Ok(ResolvedPrefix::Unix(dir))
            }
            _ => Err(InstallerErrorKind::UnsupportedOsFamily.into()),
        }
    }

    fn resolve_system_prefix(&mut self) -> Result<ResolvedPrefix, InstallerError> {
        match std::env::consts::FAMILY {
            "windows" => {
                let dir = self.get_env_var("PROGRAMFILES")?;
                let mut dir = PathBuf::from(dir);
                dir.push(&self.app_id);

                Ok(ResolvedPrefix::SingleDir(dir))
            }
            "unix" => Ok(ResolvedPrefix::Unix(PathBuf::from("/usr/local"))),
            _ => Err(InstallerErrorKind::UnsupportedOsFamily.into()),
        }
    }

    /// Returns a directory containing this package's binaries.
    pub fn bin_dir(&self) -> PathBuf {
        match &self.prefix {
            ResolvedPrefix::SingleDir(path) => path.join("bin"),
            ResolvedPrefix::Unix(path) => path.join("bin"),
        }
    }

    /// Returns a directory containing this package's data files.
    pub fn data_dir(&self) -> PathBuf {
        match &self.prefix {
            ResolvedPrefix::SingleDir(path) => path.to_path_buf(),
            ResolvedPrefix::Unix(path) => path.join("share").join(&self.app_id),
        }
    }

    fn get_env_var<K: AsRef<OsStr>>(&self, key: K) -> Result<OsString, InstallerError> {
        if let Some(map) = &self.env_map {
            map.get(key.as_ref())
                .cloned()
                .ok_or_else(|| InstallerErrorKind::InvalidEnvironmentVariable.into())
        } else {
            crate::os::env_var(key)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn get_env_map() -> HashMap<OsString, OsString> {
        HashMap::from_iter(
            [
                ("LOCALAPPDATA", "c:/users/rust/appdata/local"),
                ("PROGRAMFILES", "c:/program files"),
                ("XDG_DATA_HOME", "/home/rust/.local/share"),
                ("HOME", "/home/rust"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into())),
        )
    }

    #[cfg(windows)]
    #[test]
    fn test_user_windows() {
        let resolver =
            PathResolver::new_impl("my_app", &AppPathPrefix::User, Some(get_env_map())).unwrap();

        let bin_dir = resolver.bin_dir();

        assert!(bin_dir.is_absolute());
        assert_eq!(
            bin_dir,
            Path::new("c:/users/rust/appdata/local/Programs/my_app/bin")
        );

        let data_dir = resolver.data_dir();

        assert!(data_dir.is_absolute());
        assert_eq!(
            data_dir,
            Path::new("c:/users/rust/appdata/local/Programs/my_app")
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_user_unix() {
        let resolver =
            PathResolver::new_impl("my_app", &AppPathPrefix::User, Some(get_env_map())).unwrap();

        let bin_dir = resolver.bin_dir();

        assert!(bin_dir.is_absolute());
        assert_eq!(bin_dir, Path::new("/home/rust/.local/bin"));

        let data_dir = resolver.data_dir();

        assert!(data_dir.is_absolute());
        assert_eq!(data_dir, Path::new("/home/rust/.local/share/my_app"));
    }

    #[cfg(windows)]
    #[test]
    fn test_system_windows() {
        let resolver =
            PathResolver::new_impl("my_app", &AppPathPrefix::System, Some(get_env_map())).unwrap();

        let bin_dir = resolver.bin_dir();

        assert_eq!(bin_dir, Path::new("c:/program files/my_app/bin"));

        let data_dir = resolver.data_dir();

        assert_eq!(data_dir, Path::new("c:/program files/my_app"));
    }

    #[cfg(unix)]
    #[test]
    fn test_system_unix() {
        let resolver =
            PathResolver::new_impl("my_app", &AppPathPrefix::System, Some(get_env_map())).unwrap();

        let bin_dir = resolver.bin_dir();

        assert_eq!(bin_dir, Path::new("/usr/local/bin"));

        let data_dir = resolver.data_dir();

        assert_eq!(data_dir, Path::new("/usr/local/share/my_app"));
    }

    #[test]
    fn test_single_dir() {
        let resolver = PathResolver::new_impl(
            "my_app",
            &AppPathPrefix::SingleDir(PathBuf::from("/opt/my_app")),
            Some(get_env_map()),
        )
        .unwrap();

        let bin_dir = resolver.bin_dir();

        assert_eq!(bin_dir, Path::new("/opt/my_app/bin"));

        let data_dir = resolver.data_dir();

        assert_eq!(data_dir, Path::new("/opt/my_app"));
    }

    #[test]
    fn test_custom_unix() {
        let resolver = PathResolver::new_impl(
            "my_app",
            &AppPathPrefix::CustomUnix(PathBuf::from("/usr2")),
            Some(get_env_map()),
        )
        .unwrap();

        let bin_dir = resolver.bin_dir();

        assert_eq!(bin_dir, Path::new("/usr2/bin"));

        let data_dir = resolver.data_dir();

        assert_eq!(data_dir, Path::new("/usr2/share/my_app"));
    }
}
