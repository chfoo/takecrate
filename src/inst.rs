//! Installer functionality.
//!
//! ## Additional files
//!
//! Sometimes, you may need to include additional files beside the binary.
//! This crate does not implement self-extracting archives. However, you
//! can still bundle your files into a zip or tar.gz file and instruct the
//! user to extract them.
//!
//! Additional files example:
//!
//! ```
//! # use takecrate::inst::PackageManifest;
//! # use takecrate::manifest::FileType;
//! # use takecrate::manifest::AppId;
//! # let app_id = AppId::new("com.example.my_app").unwrap();
//! let manifest = PackageManifest::new(&app_id)
//!     .with_self_exe()
//!     .unwrap()
//!     .with_file_entry("my_data_file.dat", FileType::Data)
//!     .unwrap()
//!     .with_file_entry("another_data_file.dat", FileType::Data)
//!     .unwrap();
//! ```
use std::cell::RefCell;
use std::rc::Rc;

use exec::Executor;
use plan::Planner;

use crate::error::{InstallerError, InstallerErrorKind};
use crate::os::AccessScope;
use crate::tui::{GuidedDialogButton, Tui};

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
    tui: Rc<RefCell<Tui>>,
}

impl Installer {
    /// Create a new installer for the given package.
    pub fn new(package_manifest: &PackageManifest) -> Self {
        Self {
            package_manifest: package_manifest.clone(),
            tui: Rc::new(RefCell::new(Tui::new(
                package_manifest
                    .app_metadata
                    .get_display_name(&crate::locale::current_lang_tag()),
                &package_manifest.app_metadata.display_version,
            ))),
        }
    }

    /// Install with a TUI.
    pub fn run_interactive(&mut self) -> Result<(), InstallerError> {
        self.tui.borrow_mut().run_background();

        let result = self.run_interactive_impl();

        if let Err(error) = &result {
            if !matches!(error.kind(), InstallerErrorKind::InterruptedByUser) {
                self.tui.borrow_mut().show_error(error)?;
            }
        }

        self.tui.borrow_mut().stop()?;

        result
    }

    fn run_interactive_impl(&mut self) -> Result<(), InstallerError> {
        let mut config = InstallConfig {
            source_dir: crate::os::current_exe_dir()?,
            ..Default::default()
        };

        let tui = self.tui.borrow_mut();

        self.package_manifest.verify(&config.source_dir)?;
        tui.installation_intro()?.unwrap_button()?;
        config.access_scope = tui.prompt_access_scope()?.unwrap_button()?;
        config.destination = config.access_scope.into();

        // Modifying system search path on Unix not supported and likely
        // not necessary.
        if cfg!(windows) || config.access_scope == AccessScope::User {
            config.modify_os_search_path = tui.prompt_modify_search_path()?.unwrap_button()?;
        }

        tui.prompt_install_confirm()?.unwrap_button()?;
        tui.show_install_progress_dialog()?;

        drop(tui);
        self.run_impl(&config)?;

        let tui = self.tui.borrow_mut();

        tui.hide_install_progress_dialog()?;
        tui.installation_conclusion()?;

        Ok(())
    }

    /// Install automatically.
    pub fn run(&mut self, config: &InstallConfig) -> Result<(), InstallerError> {
        self.package_manifest.verify(&config.source_dir)?;
        self.run_impl(config)
    }

    fn run_impl(&mut self, config: &InstallConfig) -> Result<(), InstallerError> {
        tracing::debug!(package_manifest = ?self.package_manifest, ?config, "running planner");

        let mut planner = Planner::new(&self.package_manifest, config);
        let plan = planner.run()?;

        tracing::debug!(?plan, "created plan");

        let tui = self.tui.clone();
        let mut executor = Executor::new(&self.package_manifest.app_id, &plan)
            .with_progress_callback(move |current, total| {
                if tui.borrow().is_running() {
                    let _ = tui.borrow_mut().update_install_progress(current, total);
                }
            });

        executor.run()?;

        Ok(())
    }
}

impl<T> GuidedDialogButton<T> {
    fn unwrap_button(self) -> Result<T, InstallerError> {
        match self {
            GuidedDialogButton::Exit => Err(InstallerErrorKind::InterruptedByUser.into()),
            GuidedDialogButton::Next(value) => Ok(value),
        }
    }
}
