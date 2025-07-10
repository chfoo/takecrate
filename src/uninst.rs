//! Uninstaller functionality.

use std::{cell::RefCell, rc::Rc};

#[cfg(feature = "ui")]
use crate::tui::Tui;
use crate::{
    error::{AddContext, AddInstallerContext, InstallerError, InstallerErrorKind},
    manifest::{AppId, DiskManifest},
};

/// The uninstaller interface.
#[derive(Debug)]
pub struct Uninstaller {
    app_id: AppId,
    manifest: DiskManifest,
    manual_manifest: Option<DiskManifest>,
    #[cfg(feature = "ui")]
    tui: Rc<RefCell<Tui>>,
}

impl Uninstaller {
    /// Creates a new uninstaller.
    ///
    /// `app_id` is the application ID of the current binary.
    pub fn new(app_id: &AppId) -> Self {
        Self {
            app_id: app_id.clone(),
            #[cfg(feature = "ui")]
            tui: Rc::new(RefCell::new(Tui::new())),
            manifest: Default::default(),
            manual_manifest: None,
        }
    }

    /// Manually specify a disk manifest instead of discovering it.
    pub fn with_manifest(mut self, manifest: &DiskManifest) -> Self {
        self.manual_manifest = Some(manifest.clone());
        self
    }

    /// Sets the BCP 47 language tag used for the UI.
    #[cfg(feature = "ui")]
    pub fn with_language_tag(self, value: String) -> Self {
        self.tui.borrow_mut().set_lang_tag(&value);
        self
    }

    /// Sets the theme for the UI.
    #[cfg(feature = "ui-theme")]
    pub fn with_theme(self, value: cursive::theme::Theme) -> Self {
        self.tui.borrow_mut().set_theme(value);
        self
    }

    /// Sets whether this library's branding is enabled in the UI.
    ///
    /// Default is `true`.
    #[cfg(feature = "ui")]
    pub fn with_branding(self, value: bool) -> Self {
        self.tui.borrow_mut().set_enable_branding(value);
        self
    }

    #[cfg(feature = "ui")]
    // To be called from the installer only
    pub(crate) fn with_tui(mut self, tui: Rc<RefCell<Tui>>) -> Self {
        self.tui = tui;
        self
    }

    /// Uninstall with a TUI.
    #[cfg(feature = "ui")]
    pub fn run_interactive(&mut self) -> Result<(), InstallerError> {
        self.tui.borrow_mut().run_background();

        let result = self.run_interactive_impl();

        if let Err(error) = &result {
            match error.kind() {
                InstallerErrorKind::NotInstalled => {
                    self.tui.borrow_mut().show_unneeded_install(true)?;
                }
                InstallerErrorKind::InterruptedByUser => {}
                _ => {
                    self.tui.borrow_mut().show_error(error)?;
                }
            }
        }

        self.tui.borrow_mut().stop()?;

        result
    }

    #[cfg(feature = "ui")]
    fn run_interactive_impl(&mut self) -> Result<(), InstallerError> {
        use std::time::Duration;

        self.tui.borrow().set_up_background_text(true)?;

        self.discover_manifest()?;

        let mut tui = self.tui.borrow_mut();

        tui.set_name(&self.manifest.app_name, &self.manifest.app_version);

        tui.uninstallation_intro()?.unwrap_button()?;
        tui.show_uninstall_progress_dialog()?;

        drop(tui);
        self.run_impl()?;

        let tui = self.tui.borrow_mut();

        // As described in the installer, pause briefly so the user can see we did something.
        std::thread::sleep(Duration::from_millis(500));

        tui.hide_uninstall_progress_dialog()?;
        tui.uninstallation_conclusion()?;

        Ok(())
    }

    /// Automatically uninstall the binary.
    pub fn run(&mut self) -> Result<(), InstallerError> {
        self.discover_manifest()?;
        self.run_impl()
    }

    #[cfg(feature = "ui")]
    // To be called from the installer only
    pub(crate) fn run_from_installer_interactive(&mut self) -> Result<(), InstallerError> {
        self.discover_manifest()?;
        self.tui.borrow_mut().show_uninstall_progress_dialog()?;
        self.run_impl()?;
        self.tui.borrow_mut().hide_uninstall_progress_dialog()?;

        Ok(())
    }

    fn run_impl(&mut self) -> Result<(), InstallerError> {
        self.verify_matching_manifest()?;
        self.remove_app_path()
            .inst_context("failed to remove App Path")?;
        self.remove_path_env_var()
            .inst_context("failed to remove PATH environment variable")?;
        self.remove_files()?;
        self.remove_self()
            .inst_context("failed to remove self executable")?;
        self.remove_manifest_file()
            .inst_context("failed to remove manifest file")?;
        self.remove_dirs()?;
        self.remove_uninstall_entry()
            .inst_context("failed to remove uninstall entry")?;

        Ok(())
    }

    fn discover_manifest(&mut self) -> Result<(), InstallerError> {
        if let Some(manifest) = self.manual_manifest.take() {
            self.manifest = manifest;
        } else {
            self.manifest = crate::manifest(&self.app_id).map_err(|error| {
                if matches!(error.kind(), InstallerErrorKind::DiskManifestNotFound) {
                    InstallerError::new(InstallerErrorKind::NotInstalled).with_source(error)
                } else {
                    error
                }
            })?;
        }

        Ok(())
    }

    fn verify_matching_manifest(&self) -> Result<(), InstallerError> {
        tracing::info!("verify matching manifest");

        if self.manifest.app_id.uuid() != self.app_id.uuid() {
            Err(InstallerErrorKind::MismatchedDiskManifest.into())
        } else {
            Ok(())
        }
    }

    fn remove_app_path(&self) -> Result<(), InstallerError> {
        #[cfg(windows)]
        {
            if let Some(exe_name) = &self.manifest.app_path_exe_name {
                tracing::info!(exe_name, "remove app path");

                crate::os::windows::remove_app_path(self.manifest.access_scope, exe_name)?;
            }
        }
        Ok(())
    }
    fn remove_path_env_var(&self) -> Result<(), InstallerError> {
        #[cfg(windows)]
        {
            if let Some(exe_dir) = &self.manifest.search_path {
                tracing::info!(?exe_dir, "remove PATH environment variable");

                crate::os::windows::remove_path_env_var(
                    self.manifest.access_scope,
                    exe_dir.as_os_str(),
                )?;
            }
        }
        #[cfg(unix)]
        {
            if let Some(exe_dir) = &self.manifest.search_path {
                if let Some(profile) = &self.manifest.shell_profile_path {
                    let profile = profile.clone();
                    tracing::info!(?exe_dir, ?profile, "remove PATH environment variable");

                    crate::os::unix::remove_path_env_var(
                        self.manifest.access_scope,
                        exe_dir.as_os_str(),
                        &profile,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn remove_uninstall_entry(&self) -> Result<(), InstallerError> {
        #[cfg(windows)]
        {
            tracing::info!("remove application uninstall entry");

            crate::os::windows::remove_uninstall_entry(
                self.manifest.access_scope,
                &self.manifest.app_id,
            )?;
        }
        Ok(())
    }

    fn remove_files(&self) -> Result<(), InstallerError> {
        let mut current = 0;
        let total = self.manifest.total_file_size();

        for entry in &self.manifest.files {
            if entry.is_main_executable {
                continue;
            }

            if entry.path.exists() {
                let checksum = crate::os::file_checksum(&entry.path).with_contextc(|_e| {
                    format!("failed to read checksum for file {:?}", entry.path)
                })?;

                if checksum.crc32c != entry.crc32c {
                    tracing::warn!(path = ?entry.path, "cannot remove file: is modified");
                    continue;
                }

                tracing::info!(path = ?entry.path, "removing file");
                std::fs::remove_file(&entry.path)
                    .with_contextc(|_e| format!("failed to remove file {:?}", entry.path))?;
            } else {
                tracing::warn!(path = ?entry.path, "cannot remove file: is missing");
            }

            current += entry.len;

            #[cfg(feature = "ui")]
            if self.tui.borrow().is_running() {
                self.tui
                    .borrow_mut()
                    .update_uninstall_progress(current, total)?;
            }
        }

        Ok(())
    }

    fn remove_dirs(&self) -> Result<(), InstallerError> {
        for entry in &self.manifest.dirs {
            if !entry.preserve {
                if entry.path.exists() {
                    if std::fs::read_dir(&entry.path)?.count() == 0 {
                        tracing::info!(path = ?entry.path, "removing directory");

                        std::fs::remove_dir(&entry.path).with_contextc(|_e| {
                            format!("failed to remove directory {:?}", entry.path)
                        })?;
                    } else {
                        tracing::warn!(path = ?entry.path, "cannot remove directory: not empty");
                    }
                } else {
                    tracing::warn!(path = ?entry.path, "cannot remove directory: is missing");
                }
            }
        }

        Ok(())
    }

    fn remove_manifest_file(&self) -> Result<(), InstallerError> {
        tracing::info!(path = ?&self.manifest.manifest_path, "removing manifest file");

        std::fs::remove_file(&self.manifest.manifest_path)?;

        Ok(())
    }

    fn remove_self(&self) -> Result<(), InstallerError> {
        if let Some(entry) = self
            .manifest
            .files
            .iter()
            .find(|entry| entry.is_main_executable)
        {
            if entry.path.exists() {
                let checksum = crate::os::file_checksum(&entry.path)?;

                if checksum.crc32c != entry.crc32c {
                    tracing::warn!(path = ?entry.path, "cannot remove file: is modified");
                    return Ok(());
                }

                tracing::info!(path = ?&entry.path, "removing self executable");

                self_replace::self_delete_at(&entry.path)?;
            } else {
                tracing::warn!(path = ?&entry.path, "self executable not found");
            }
        } else {
            tracing::warn!("manifest has no self executable");
        }

        Ok(())
    }
}
