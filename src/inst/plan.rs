use std::{ffi::OsString, path::PathBuf};

use crate::{
    error::{AddContext, InstallerError},
    manifest::FileType,
    os::AccessScope,
    path::{AppPathPrefix, PathResolver},
};

use super::{InstallConfig, PackageManifest};

#[derive(Debug, Clone, Default)]
pub struct InstallPlan {
    pub display_name: String,
    pub display_version: String,
    pub access_scope: AccessScope,
    pub manifest_path: PathBuf,
    pub destination: AppPathPrefix,
    pub dirs: Vec<PlanDirEntry>,
    pub files: Vec<PlanFileEntry>,
    pub search_path: Option<OsString>,
    #[cfg(windows)]
    pub app_path: Option<PlanAppPath>,
    #[cfg(unix)]
    pub shell_profile_path: Option<PathBuf>,
    #[cfg(windows)]
    pub interactive_uninstall_args: OsString,
    #[cfg(windows)]
    pub quiet_uninstall_args: OsString,
}

impl InstallPlan {
    pub fn main_executable(&self) -> Option<&PlanFileEntry> {
        self.files.iter().find(|entry| entry.is_main_executable)
    }

    pub fn total_file_size(&self) -> u64 {
        self.files.iter().map(|entry| entry.len).sum()
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Default)]
pub struct PlanAppPath {
    pub exe_name: String,
    pub exe_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PlanDirEntry {
    pub destination_path: PathBuf,
    pub preserve: bool,
}

#[derive(Debug, Clone)]
pub struct PlanFileEntry {
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub file_type: FileType,
    pub is_main_executable: bool,
    pub len: u64,
    pub crc32c: u32,
    #[cfg(unix)]
    pub posix_permissions: u32,
}

#[derive(Debug)]
pub struct Planner {
    package_manifest: PackageManifest,
    config: InstallConfig,
}

impl Planner {
    pub fn new(package_manifest: &PackageManifest, config: &InstallConfig) -> Self {
        Self {
            package_manifest: package_manifest.clone(),
            config: config.clone(),
        }
    }

    pub fn run(&mut self) -> Result<InstallPlan, InstallerError> {
        let display_name = self.package_manifest.app_metadata.display_name.clone();
        let display_version = self.package_manifest.app_metadata.display_version.clone();
        let manifest_path = crate::manifest::manifest_path(
            &self.package_manifest.app_id,
            self.config.access_scope,
        )?;

        let mut plan = InstallPlan {
            display_name,
            display_version,
            access_scope: self.config.access_scope,
            manifest_path,
            destination: self.config.destination.clone(),
            dirs: Default::default(),
            files: Default::default(),
            search_path: None,
            #[cfg(windows)]
            app_path: None,
            #[cfg(unix)]
            shell_profile_path: None,
            #[cfg(windows)]
            interactive_uninstall_args: OsString::from(
                self.package_manifest.interactive_uninstall_args.join(" "),
            ),
            #[cfg(windows)]
            quiet_uninstall_args: OsString::from(
                self.package_manifest.quiet_uninstall_args.join(" "),
            ),
        };
        let path_resolver = PathResolver::new(
            self.package_manifest.app_id.plain_id(),
            &self.config.destination,
        )?;

        let dest_bin_dir = path_resolver.bin_dir();
        let dest_data_dir = path_resolver.data_dir();

        if self.config.modify_os_search_path {
            plan.search_path = Some(dest_bin_dir.as_os_str().to_os_string());

            #[cfg(unix)]
            {
                plan.shell_profile_path = Some(crate::os::unix::get_curent_shell_profile()?);
            }
        }

        plan.dirs.push(PlanDirEntry {
            destination_path: dest_bin_dir.clone(),
            preserve: dest_bin_dir.exists(),
        });

        plan.dirs.push(PlanDirEntry {
            destination_path: dest_data_dir.clone(),
            preserve: dest_data_dir.exists(),
        });

        for entry in &self.package_manifest.files {
            let span =
                tracing::debug_span!("planner file entry", package_path = ?entry.package_path());
            let _guard = span.enter();

            let source_path = self.config.source_dir.join(entry.package_path());

            let destination_path = match entry.file_type() {
                FileType::Executable => dest_bin_dir.join(entry.target_path()),
                FileType::Library => unimplemented!(),
                FileType::Configuration => unimplemented!(),
                FileType::Documentation => unimplemented!(),
                FileType::Data => dest_data_dir.join(entry.target_path()),
            };

            tracing::debug!(?source_path, ?destination_path, "computed paths");

            let checksum =
                crate::os::file_checksum(self.config.source_dir.join(entry.package_path()))
                    .with_contextc(|_| format!("could not read file {:?}", entry.package_path()))?;
            #[cfg(unix)]
            let posix_permissions =
                crate::os::unix::get_effective_posix_permission(entry.file_type());

            plan.files.push(PlanFileEntry {
                source_path,
                destination_path: destination_path.clone(),
                file_type: entry.file_type(),
                is_main_executable: entry.is_main_executable(),
                len: checksum.len,
                crc32c: checksum.crc32c,
                #[cfg(unix)]
                posix_permissions,
            });

            #[cfg(windows)]
            if entry.is_main_executable() && self.config.modify_os_search_path {
                let exe_name = entry
                    .target_path()
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                plan.app_path = Some(PlanAppPath {
                    exe_name,
                    exe_path: destination_path.clone(),
                });
            }
        }

        Ok(plan)
    }
}
