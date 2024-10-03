use std::{
    ffi::OsString,
    fs::File,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::{
    error::{AddContext, InstallerError, InstallerErrorKind},
    os::AccessScope,
    path::AppPathPrefix,
};

use super::AppId;

/// A category of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// A program file that can be run by the user.
    Executable,

    /// Reserved.
    ///
    /// Additional executable code used by a program.
    Library,

    /// Reserved.
    ///
    /// User editable configuration file.
    Configuration,

    /// Reserved.
    ///
    /// Documentation for the user.
    Documentation,

    /// Additional data files used by a program.
    Data,
}

impl Default for FileType {
    fn default() -> Self {
        Self::Data
    }
}

/// Information about an installed file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DiskFileEntry {
    /// Full location of the file.
    pub path: PathBuf,
    /// Size of file in bytes.
    pub len: u64,
    /// A CRC32C checksum of the file.
    pub crc32c: u32,
    /// File type.
    pub file_type: FileType,
    /// Whether this file is the main binary with the self-installer.
    pub is_main_executable: bool,
}

/// Information about an installed directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DiskDirEntry {
    /// Full path of the directory.
    pub path: PathBuf,
    /// Whether to always keep this directory when uninstalling.
    pub preserve: bool,
}

/// Details about an installed application and its files.
///
/// For the installer counterpart, see [`PackageManifest`](crate::inst::PackageManifest).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskManifest {
    #[serde(skip)]
    /// Path of manifest installed on disk.
    pub manifest_path: PathBuf,
    /// Reserved.
    pub manifest_version: u64,
    /// Application ID.
    pub app_id: AppId,
    /// Application name displayed to the user.
    pub app_name: String,
    /// Application version displayed to the user.
    pub app_version: String,
    /// Access scope.
    pub access_scope: AccessScope,
    /// Installed application path type.
    pub app_path_prefix: AppPathPrefix,
    /// Directory entries.
    pub dirs: Vec<DiskDirEntry>,
    /// File entries.
    pub files: Vec<DiskFileEntry>,
    /// If specified, the search path (PATH) installed.
    pub search_path: Option<OsString>,
    /// The filename used for the App Paths entry.
    #[cfg(any(windows, doc))]
    pub app_path_exe_name: Option<String>,
}

impl DiskManifest {
    /// Deserialize from the given path.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, InstallerError> {
        let path = path.as_ref();
        let buf = std::fs::read(path)
            .with_contextc(|_error| format!("could not open file {:?}", path))?;
        let mut manifest = Self::from_reader(Cursor::new(buf))?;

        manifest.manifest_path = path.to_path_buf();

        Ok(manifest)
    }

    /// Deserialize from the given reader.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, InstallerError> {
        let manifest = ron::de::from_reader::<R, Self>(reader).map_err(|error| {
            InstallerError::new(InstallerErrorKind::MalformedDiskManifest).with_source(error)
        })?;

        Ok(manifest)
    }

    /// Serialize to the given path.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), InstallerError> {
        let file = File::options()
            .write(true)
            .create_new(true)
            .truncate(true)
            .open(path)?;

        self.to_writer(file)?;

        Ok(())
    }

    /// Serialize to the given writer.
    pub fn to_writer<W: Write>(&self, output: W) -> Result<(), InstallerError> {
        ron::ser::to_writer_pretty(output, &self, PrettyConfig::default())
            .map_err(|error| InstallerError::new(InstallerErrorKind::Other).with_source(error))?;

        Ok(())
    }

    /// Returns the sum of `len` of the file entries.
    pub fn total_file_size(&self) -> u64 {
        self.files.iter().map(|entry| entry.len).sum()
    }
}
