//! Installer functionality.
use std::cell::RefCell;
use std::rc::Rc;

use exec::Executor;
use plan::{InstallPlan, Planner};

use crate::error::{InstallerError, InstallerErrorKind};
use crate::os::AccessScope;
#[cfg(feature = "ui")]
use crate::tui::Tui;

pub use self::config::*;
pub use self::package::*;

mod config;
mod exec;
mod package;
mod plan;

/// The installer interface.
#[derive(Debug)]
pub struct Installer {
    package_manifest: PackageManifest,
    #[cfg(feature = "ui")]
    tui: Rc<RefCell<Tui>>,
    lang_tag: String,
    plan: Option<InstallPlan>,
}

impl Installer {
    /// Create a new installer for the given package.
    pub fn new(package_manifest: &PackageManifest) -> Self {
        Self {
            package_manifest: package_manifest.clone(),
            #[cfg(feature = "ui")]
            tui: Rc::new(RefCell::new(Tui::new())),
            lang_tag: String::new(),
            plan: None,
        }
    }

    /// Sets the BCP 47 language tag used for the UI.
    #[cfg(feature = "ui")]
    pub fn with_lang_tag(mut self, lang_tag: String) -> Self {
        self.tui.borrow_mut().set_lang_tag(&lang_tag);
        self.lang_tag = lang_tag;
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

    /// Install with a TUI.
    #[cfg(feature = "ui")]
    pub fn run_interactive(&mut self) -> Result<(), InstallerError> {
        self.tui.borrow_mut().set_name(
            self.package_manifest
                .app_metadata
                .get_display_name(&self.detect_lang_tag()),
            &self.package_manifest.app_metadata.display_version,
        );
        self.tui.borrow_mut().run_background();

        let result = self.run_interactive_impl();

        if let Err(error) = &result {
            match error.kind() {
                InstallerErrorKind::AlreadyInstalled => {
                    self.tui.borrow_mut().show_unneeded_install(false)?;
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

    fn detect_lang_tag(&self) -> String {
        if !self.lang_tag.is_empty() {
            return self.lang_tag.clone();
        }

        #[cfg(feature = "i18n")]
        {
            crate::locale::current_lang_tag()
        }
        #[cfg(not(feature = "i18n"))]
        {
            String::new()
        }
    }

    #[cfg(feature = "ui")]
    fn run_interactive_impl(&mut self) -> Result<(), InstallerError> {
        use std::time::Duration;

        let mut config = InstallConfig {
            source_dir: crate::os::current_exe_dir()?,
            ..Default::default()
        };

        {
            let tui = self.tui.borrow_mut();

            tui.set_up_background_text(false)?;

            self.package_manifest.verify(&config.source_dir)?;
            tui.installation_intro()?.unwrap_button()?;
            config.access_scope = tui.prompt_access_scope()?.unwrap_button()?;
            config.destination = config.access_scope.into();

            // Modifying system search path on Unix not supported and likely
            // not necessary.
            if cfg!(windows) || config.access_scope == AccessScope::User {
                config.modify_os_search_path = tui.prompt_modify_search_path()?.unwrap_button()?;
            }
        }

        self.run_planner(&config)?;
        let uninstall_required = self.plan.as_ref().unwrap().manifest_path.exists();

        {
            let tui = self.tui.borrow_mut();

            if uninstall_required {
                tui.prompt_uninstall_existing()?.unwrap_button()?;
            }

            tui.prompt_install_confirm()?.unwrap_button()?;
        }

        self.run_uninstaller_interactive()?;

        self.tui.borrow_mut().show_install_progress_dialog()?;

        if uninstall_required {
            // Because it happens so fast, the user might be confused if they
            // see brief glimpse of only the uninstaller progress bar.
            // A visual pause is needed so that they can see the installer
            // progress bar.
            std::thread::sleep(Duration::from_millis(500));
        }

        self.run_executor()?;

        // As described above, pause briefly so the user can see we did something.
        std::thread::sleep(Duration::from_millis(500));

        let tui = self.tui.borrow_mut();

        tui.hide_install_progress_dialog()?;
        tui.installation_conclusion()?;

        Ok(())
    }

    /// Install automatically.
    pub fn run(&mut self, config: &InstallConfig) -> Result<(), InstallerError> {
        self.package_manifest.verify(&config.source_dir)?;
        self.run_planner(config)?;
        self.run_uninstaller()?;
        self.run_executor()?;
        Ok(())
    }

    fn run_planner(&mut self, config: &InstallConfig) -> Result<(), InstallerError> {
        tracing::debug!(package_manifest = ?self.package_manifest, ?config, "running planner");

        let mut planner = Planner::new(&self.package_manifest, config);
        let plan = planner.run()?;

        tracing::debug!(?plan, "created plan");
        self.plan = Some(plan);

        Ok(())
    }

    #[cfg(feature = "ui")]
    fn run_uninstaller_interactive(&mut self) -> Result<(), InstallerError> {
        let manifest_path = &self.plan.as_ref().unwrap().manifest_path;
        let uninstall_required = manifest_path.exists();

        if !uninstall_required {
            return Ok(());
        }

        let manifest = crate::manifest::DiskManifest::load(manifest_path)?;

        let mut uninstaller = crate::uninst::Uninstaller::new(&manifest.app_id)
            .with_manifest(&manifest)
            .with_tui(self.tui.clone());

        uninstaller.run_from_installer_interactive()?;

        Ok(())
    }

    fn run_uninstaller(&mut self) -> Result<(), InstallerError> {
        let manifest_path = &self.plan.as_ref().unwrap().manifest_path;
        let uninstall_required = manifest_path.exists();

        if !uninstall_required {
            return Ok(());
        }

        let manifest = crate::manifest::DiskManifest::load(manifest_path)?;

        let mut uninstaller =
            crate::uninst::Uninstaller::new(&manifest.app_id).with_manifest(&manifest);

        uninstaller.run()?;

        Ok(())
    }

    fn run_executor(&mut self) -> Result<(), InstallerError> {
        let plan = self.plan.as_ref().unwrap();
        let mut executor = Executor::new(&self.package_manifest.app_id, plan);

        #[cfg(feature = "ui")]
        if self.tui.borrow().is_running() {
            let tui = self.tui.clone();
            if tui.borrow().is_running() {
                executor = executor.with_progress_callback(move |current, total| {
                    let _ = tui.borrow_mut().update_install_progress(current, total);
                });
            }
        }

        executor.run()?;

        Ok(())
    }
}
