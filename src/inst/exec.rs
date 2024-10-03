use std::{io::Write, path::Path};

use crate::{
    error::{AddInstallerContext, InstallerError, InstallerErrorKind},
    manifest::{AppId, DiskDirEntry, DiskFileEntry, DiskManifest},
    os::FileChecksum,
};

use super::plan::{InstallPlan, PlanFileEntry};

pub struct Executor {
    app_id: AppId,
    plan: InstallPlan,
    progress_callback: Box<dyn FnMut(u64, u64)>,
}

impl Executor {
    pub fn new(app_id: &AppId, plan: &InstallPlan) -> Self {
        Self {
            app_id: app_id.clone(),
            plan: plan.clone(),
            progress_callback: Box::new(|_, _| {}),
        }
    }

    pub fn with_progress_callback<F>(mut self, progress_callback: F) -> Self
    where
        F: FnMut(u64, u64) + 'static,
    {
        self.progress_callback = Box::new(progress_callback);
        self
    }

    pub fn run(&mut self) -> Result<(), InstallerError> {
        let disk_manifest = self.populate_disk_manifest();

        self.check_existing_manifest()?;
        self.persist_disk_manifest(&disk_manifest)
            .inst_context("failed to persist disk manifest")?;
        self.copy_files()?;
        self.add_path_env_var()
            .inst_context("failed to add PATH environment variable")?;
        self.add_app_path().inst_context("failed to add App Path")?;
        self.add_uninstall_entry()
            .inst_context("failed to add uninstall entry")?;

        Ok(())
    }

    fn populate_disk_manifest(&self) -> DiskManifest {
        let mut disk_manifest = DiskManifest {
            manifest_version: 0,
            manifest_path: Default::default(),
            app_id: self.app_id.clone(),
            app_name: self.plan.display_name.clone(),
            app_version: self.plan.display_version.clone(),
            access_scope: self.plan.access_scope,
            app_path_prefix: self.plan.destination.clone(),
            dirs: Default::default(),
            files: Default::default(),
            search_path: self.plan.search_path.clone(),
            #[cfg(windows)]
            app_path_exe_name: self.plan.app_path.clone().map(|item| item.exe_name),
        };

        for entry in &self.plan.dirs {
            disk_manifest.dirs.push(DiskDirEntry {
                path: entry.destination_path.clone(),
                preserve: entry.preserve,
            });
        }

        for entry in &self.plan.files {
            disk_manifest.files.push(DiskFileEntry {
                path: entry.destination_path.clone(),
                len: entry.len,
                crc32c: entry.crc32c,
                file_type: entry.file_type,
                is_main_executable: entry.is_main_executable,
            });
        }

        disk_manifest
    }

    fn check_existing_manifest(&self) -> Result<(), InstallerError> {
        if self.plan.manifest_path.exists() {
            Err(InstallerErrorKind::AlreadyInstalled.into())
        } else {
            Ok(())
        }
    }

    fn persist_disk_manifest(&self, disk_manifest: &DiskManifest) -> Result<(), InstallerError> {
        tracing::debug!("persist disk manifest");

        let mut manifest_temp_file = tempfile::NamedTempFile::new()?;
        disk_manifest.to_writer(&mut manifest_temp_file)?;
        manifest_temp_file.flush()?;

        let manifest_checksum = crate::os::file_checksum(manifest_temp_file.path())?;

        self.copy_file(
            manifest_temp_file.path(),
            &manifest_checksum,
            &self.plan.manifest_path,
        )?;
        #[cfg(unix)]
        {
            use crate::error::AddContext;
            let mode =
                crate::os::unix::get_effective_posix_permission(crate::manifest::FileType::Data);
            crate::os::unix::set_posix_permission(&self.plan.manifest_path, mode)
                .with_context("failed to set disk manifest file permissions")?;
        }

        Ok(())
    }

    fn copy_files(&mut self) -> Result<(), InstallerError> {
        let mut current = 0;
        let total = self.plan.total_file_size();

        for entry in &self.plan.files {
            let span =
                tracing::debug_span!("executor file entry", source_path = ?entry.source_path);
            let _guard = span.enter();

            let checksum = FileChecksum {
                crc32c: entry.crc32c,
                len: entry.len,
            };
            self.copy_file(&entry.source_path, &checksum, &entry.destination_path)
                .inst_contextc(|| {
                    format!(
                        "failed to copy file {:?} {:?}",
                        entry.source_path, entry.destination_path
                    )
                })?;
            self.apply_posix_permission(entry).inst_contextc(|| {
                format!(
                    "failed to set file permissions {:?}",
                    entry.destination_path
                )
            })?;

            current += entry.len;
            (self.progress_callback)(current, total);
        }

        Ok(())
    }

    fn copy_file(
        &self,
        source: &Path,
        source_checksum: &FileChecksum,
        destination: &Path,
    ) -> Result<(), InstallerError> {
        if destination.exists() {
            let checksum = crate::os::file_checksum(destination)?;

            if source_checksum == &checksum {
                tracing::info!(?destination, "destination file already exists");

                return Ok(());
            } else {
                tracing::error!(?destination, "unknown file in destination");
                return Err(InstallerErrorKind::UnknownFileInDestination.into());
            }
        }

        tracing::info!(?source, ?destination, "copying file");

        if let Some(parent) = destination.parent() {
            tracing::debug!(dir = ?parent, "creating directories");
            std::fs::create_dir_all(parent)?;
        }

        std::fs::copy(source, destination)?;

        Ok(())
    }

    fn apply_posix_permission(&self, entry: &PlanFileEntry) -> Result<(), InstallerError> {
        #[cfg(unix)]
        {
            let mode = entry.posix_permissions;
            tracing::debug!(mode, ?entry.destination_path, "set POSIX permissions");

            crate::os::unix::set_posix_permission(&entry.destination_path, mode)?;
        }

        let _ = entry;

        Ok(())
    }

    fn add_path_env_var(&self) -> Result<(), InstallerError> {
        #[cfg(windows)]
        if let Some(part) = &self.plan.search_path {
            tracing::info!(?part, "modifying Path environment variable");
            crate::os::windows::add_path_env_var(self.plan.access_scope, part)?;
        }

        #[cfg(unix)]
        if let Some(part) = &self.plan.search_path {
            let profile = crate::os::unix::get_current_shell_profile()?;
            tracing::info!(?part, ?profile, "modifying PATH environment variable");
            crate::os::unix::add_path_env_var(self.plan.access_scope, &part, &profile)?;
        }
        Ok(())
    }

    fn add_app_path(&self) -> Result<(), InstallerError> {
        #[cfg(windows)]
        if let Some(app_path) = &self.plan.app_path {
            tracing::info!(name = ?app_path.exe_name, "modifying App Paths");
            let config = crate::os::windows::AppPathConfig::default();
            crate::os::windows::add_app_path(
                self.plan.access_scope,
                &app_path.exe_name,
                app_path.exe_path.as_os_str(),
                &config,
            )?;
        }

        Ok(())
    }

    fn add_uninstall_entry(&self) -> Result<(), InstallerError> {
        #[cfg(windows)]
        {
            if self.plan.interactive_uninstall_args.is_empty() {
                tracing::warn!("no uninstall arguments provided");
                return Ok(());
            }

            if let Some(entry) = self.plan.main_executable() {
                tracing::info!("adding uninstall entry");

                let config = crate::os::windows::UninstallEntryConfig {
                    manifest_path: self.plan.manifest_path.clone(),
                    display_name: self.plan.display_name.clone(),
                    display_version: self.plan.display_version.clone(),
                    publisher: String::new(),
                    estimated_size: self.plan.total_file_size(),
                    quiet_exe_args: self.plan.quiet_uninstall_args.clone(),
                };

                crate::os::windows::add_uninstall_entry(
                    self.plan.access_scope,
                    &self.app_id,
                    entry.destination_path.as_os_str(),
                    &self.plan.interactive_uninstall_args,
                    &config,
                )?;
            }
        }
        #[cfg(unix)]
        {
            let _ = self.plan.main_executable();
        }

        Ok(())
    }
}
