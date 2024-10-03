use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::{
    error::{InstallerError, InstallerErrorKind},
    manifest::{AppId, AppMetadata, FileType},
};

/// Details of the binary and any associated files to be installed.
///
/// For the installed counterpart, see [`DiskManifest`](crate::manifest::DiskManifest).
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PackageManifest {
    /// Application ID.
    pub app_id: AppId,

    /// Application metadata.
    pub app_metadata: AppMetadata,

    /// Files to be installed.
    pub files: Vec<PackageFileEntry>,

    /// Additional arguments passed to the binary to start the interactive uninstaller.
    ///
    /// This may be called by the OS application settings.
    pub interactive_uninstall_args: Vec<String>,

    /// Additional arguments passed to the binary to start the automatic uninstaller.
    ///
    /// This may be called by shell scripts.
    pub quiet_uninstall_args: Vec<String>,
}

impl PackageManifest {
    /// Creates a new package manifest.
    ///
    /// Don't forget to add an entry for the binary itself using [`Self::with_self_exe()`]
    /// or [`Self::with_self_exe_renamed()`].
    pub fn new(app_id: &AppId) -> Self {
        Self {
            app_id: app_id.clone(),
            app_metadata: AppMetadata::default(),
            files: Vec::new(),
            interactive_uninstall_args: Vec::new(),
            quiet_uninstall_args: Vec::new(),
        }
    }

    /// Adds a file entry with the current binary.
    pub fn with_self_exe(mut self) -> Result<Self, InstallerError> {
        let current_exe_name = crate::os::current_exe_name()?;

        self.files.push(PackageFileEntry::new_main_exe(
            current_exe_name.clone(),
            current_exe_name,
            FileType::Executable,
        )?);

        Ok(self)
    }

    /// Adds a file entry with the current binary and a destination name.
    ///
    /// You need to append [`std::env::consts::EXE_SUFFIX`] yourself.
    pub fn with_self_exe_renamed<S: AsRef<str>>(
        mut self,
        exe_name: S,
    ) -> Result<Self, InstallerError> {
        let current_exe_name = crate::os::current_exe_name()?;

        self.files.push(PackageFileEntry::new_main_exe(
            current_exe_name,
            exe_name.as_ref().into(),
            FileType::Executable,
        )?);

        Ok(self)
    }

    /// Sets the `interactive_uninstall_args` field.
    pub fn with_interactive_uninstall_args(mut self, args: &[&str]) -> Self {
        self.interactive_uninstall_args = args.iter().map(|arg| arg.to_string()).collect();

        self
    }

    /// Sets the `quiet_uninstall_args` field.
    pub fn with_quiet_uninstall_args(mut self, args: &[&str]) -> Self {
        self.quiet_uninstall_args = args.iter().map(|arg| arg.to_string()).collect();

        self
    }

    /// Adds a file entry.
    pub fn with_file_entry<P: AsRef<Path>>(
        mut self,
        package_path: P,
        file_type: FileType,
    ) -> Result<Self, InstallerError> {
        self.files.push(PackageFileEntry::new(
            package_path.as_ref(),
            package_path.as_ref(),
            file_type,
        )?);
        Ok(self)
    }

    /// Adds a file entry with a destination name.
    pub fn with_file_entry_renamed<P: AsRef<Path>>(
        mut self,
        package_path: P,
        target_path: P,
        file_type: FileType,
    ) -> Result<Self, InstallerError> {
        self.files
            .push(PackageFileEntry::new(package_path, target_path, file_type)?);
        Ok(self)
    }

    /// Returns the file entry containing the binary.
    pub fn main_executable(&self) -> Option<&PackageFileEntry> {
        self.files.iter().find(|entry| entry.is_main_executable)
    }

    /// Checks if the files can be read.
    ///
    /// This is intended for a quick test for basic errors.
    pub fn verify<P: AsRef<Path>>(&self, source_dir: P) -> Result<(), PackageVerifyError> {
        self.main_executable()
            .ok_or(PackageVerifyError::MissingMainExecutable)?;

        let source_dir = source_dir.as_ref();

        for entry in &self.files {
            let source_path = source_dir.join(entry.package_path());

            let _ = File::open(&source_path).map_err(|source| PackageVerifyError::InvalidFile {
                path: source_path.clone(),
                source,
            })?;
        }

        Ok(())
    }
}

/// An entry for a file in a package manifest.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PackageFileEntry {
    package_path: PathBuf,
    target_path: PathBuf,
    file_type: FileType,
    is_main_executable: bool,
}

impl PackageFileEntry {
    /// Creates a file entry.
    pub fn new<P: AsRef<Path>>(
        package_path: P,
        target_path: P,
        file_type: FileType,
    ) -> Result<Self, PackagePathError> {
        Self::new_impl(package_path, target_path, file_type, false)
    }

    /// Creates a file entry with fields populated for a main binary.
    pub fn new_main_exe<P: AsRef<Path>>(
        package_path: P,
        target_path: P,
        file_type: FileType,
    ) -> Result<Self, PackagePathError> {
        Self::new_impl(package_path, target_path, file_type, true)
    }

    fn new_impl<P: AsRef<Path>>(
        package_path: P,
        target_path: P,
        file_type: FileType,
        is_main_executable: bool,
    ) -> Result<Self, PackagePathError> {
        Self::validate_path(package_path.as_ref())?;
        Self::validate_path(target_path.as_ref())?;

        Ok(Self {
            package_path: package_path.as_ref().to_owned(),
            target_path: target_path.as_ref().to_owned(),
            file_type,
            is_main_executable,
        })
    }

    fn validate_path(path: &Path) -> Result<(), PackagePathError> {
        for component in path.components() {
            match component {
                std::path::Component::Normal(_) => continue,
                _ => return Err(PackagePathError::new(path.to_path_buf())),
            }
        }

        Ok(())
    }

    /// Returns the relative path of a source file.
    pub fn package_path(&self) -> &PathBuf {
        &self.package_path
    }

    /// Returns the relative path of a destination file.
    pub fn target_path(&self) -> &PathBuf {
        &self.target_path
    }

    /// Returns the file type.
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Returns whether the file is the main binary.
    pub fn is_main_executable(&self) -> bool {
        self.is_main_executable
    }
}

/// Error for a invalid path to a file in a package.
#[derive(Debug, thiserror::Error)]
#[error("package path error: {path}")]
pub struct PackagePathError {
    path: PathBuf,
}

impl PackagePathError {
    /// Creates a new error.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns the invalid path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl From<PackagePathError> for InstallerError {
    fn from(value: PackagePathError) -> Self {
        InstallerError::new(InstallerErrorKind::InvalidPackageManifest).with_source(value)
    }
}

/// Error for verifying a package manifest.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PackageVerifyError {
    /// I/O error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// File entry for a main binary is missing.
    #[error("missing main executable")]
    MissingMainExecutable,

    /// Invalid file.
    #[error("invalid file {path}")]
    InvalidFile {
        /// Relative path of the file.
        path: PathBuf,
        /// Source error.
        #[source]
        source: std::io::Error,
    },
}

impl From<PackageVerifyError> for InstallerError {
    fn from(value: PackageVerifyError) -> Self {
        InstallerError::new(InstallerErrorKind::InvalidPackageManifest).with_source(value)
    }
}
