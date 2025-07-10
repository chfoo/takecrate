//! Terminal user interface.

use std::{
    fmt::Debug,
    sync::{mpsc::Receiver, Arc},
    thread::JoinHandle,
};

pub use cursive;
use cursive::{
    theme::Theme,
    view::Scrollable,
    views::{
        stack_view::{Fullscreen, Transparent},
        Dialog, LinearLayout, RadioGroup, TextView,
    },
    CbSink, Cursive, CursiveExt,
};
use dialog::GuidedDialogButton;

use crate::{
    error::{InstallerError, InstallerErrorKind},
    locale::Locale,
    os::AccessScope,
};

mod bg;
mod dialog;

pub(crate) struct Tui {
    channel: Option<CbSink>,
    handle: Option<JoinHandle<std::io::Result<()>>>,
    app_name: String,
    app_version: String,
    locale: Locale,
    theme: Option<Theme>,
    enable_branding: bool,
}

impl Tui {
    pub fn new() -> Self {
        Self {
            channel: None,
            handle: None,
            app_name: String::new(),
            app_version: String::new(),
            locale: Locale::with_system(),
            theme: None,
            enable_branding: true,
        }
    }

    pub fn is_running(&self) -> bool {
        self.channel.is_some()
    }

    pub fn set_name(&mut self, app_name: &str, app_version: &str) {
        self.app_name = app_name.to_string();
        self.app_version = app_version.to_string();
    }

    pub fn set_lang_tag(&mut self, value: &str) {
        self.locale.set_language_tag(value);
    }

    #[cfg(feature = "ui-theme")]
    pub fn set_theme(&mut self, value: Theme) {
        self.theme = Some(value);
    }

    pub fn set_enable_branding(&mut self, enable_branding: bool) {
        self.enable_branding = enable_branding;
    }

    pub fn run_background(&mut self) {
        assert!(self.channel.is_none());

        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let theme = self.theme.clone();

        let join_handle = std::thread::spawn(move || {
            let mut cursive = cursive::Cursive::new();

            if let Some(theme) = theme {
                cursive.set_theme(theme);
            }

            sender.send(cursive.cb_sink().clone()).unwrap();

            cursive.run_crossterm()
        });

        self.handle = Some(join_handle);

        self.channel = Some(receiver.recv().unwrap());
    }

    pub fn stop(&mut self) -> Result<(), InstallerError> {
        let _ = self
            .channel
            .take()
            .unwrap()
            .send(Box::new(|cursive| cursive.quit()));

        self.handle.take().unwrap().join().unwrap()?;

        Ok(())
    }

    fn show_wait_dialog<T>(
        &self,
        dialog: Dialog,
        dialog_receiver: Receiver<T>,
    ) -> Result<T, InstallerError>
    where
        T: Send + 'static,
    {
        self.in_cursive(move |cursive| {
            cursive.add_layer(dialog);
        })?;

        dialog_receiver
            .recv()
            .map_err(|_error| InstallerErrorKind::Terminal.into())
    }

    pub fn set_up_background_text(&self, is_uninstall: bool) -> Result<(), InstallerError> {
        let text = if is_uninstall {
            self.locale
                .text_args("uninstaller-title", [("app_name", (&self.app_name).into())])
        } else {
            self.locale
                .text_args("installer-title", [("app_name", (&self.app_name).into())])
        };
        let subtext = self.locale.text("powered-by-library");

        let view = bg::background_text(&text, &subtext);

        self.in_cursive(|cursive| {
            cursive
                .screen_mut()
                .add_layer(Transparent(Fullscreen(view)));
        })
    }

    pub fn show_error<E>(&self, error: E) -> Result<(), InstallerError>
    where
        E: std::error::Error,
    {
        let label = self.locale.text("error-occurred");
        let details_label = self.locale.text("error-details");
        let error_message = crate::error::format_error(error);

        let layout = LinearLayout::vertical()
            .child(TextView::new(label))
            .child(TextView::new("\n"))
            .child(TextView::new(details_label))
            .child(TextView::new("\n"))
            .child(TextView::new(error_message).scrollable());

        let (mut dialog, dialog_receiver) = dialog::info_dialog(&self.locale, "");
        dialog.set_content(layout);

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn show_unneeded_install(&self, is_uninstall: bool) -> Result<(), InstallerError> {
        let title = self.locale.text("error-occurred");
        let text = if is_uninstall {
            self.locale.text("app-not-installed")
        } else {
            self.locale.text("app-already-installed")
        };

        let (mut dialog, dialog_receiver) = dialog::info_dialog(&self.locale, &title);
        dialog.set_content(TextView::new(text).scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn installation_intro(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        let args = [
            ("app_name", (&self.app_name).into()),
            ("app_version", (&self.app_version).into()),
        ];
        let text = self.locale.text_args("installer-intro", args);

        let (mut dialog, dialog_receiver) = dialog::guided_dialog(&self.locale, "", |_| ());
        dialog.set_content(TextView::new(text).scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn installation_conclusion(&self) -> Result<(), InstallerError> {
        let args = [("app_name", (&self.app_name).into())];
        let text = self.locale.text_args("installer-conclusion", args);

        let (mut dialog, dialog_receiver) = dialog::info_dialog(&self.locale, "");
        dialog.set_content(TextView::new(text).scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn prompt_access_scope(&self) -> Result<GuidedDialogButton<AccessScope>, InstallerError> {
        let mut layout = LinearLayout::vertical();
        layout.add_child(TextView::new(self.locale.text("access-scope-prompt")));

        let mut radio_group = RadioGroup::new();
        layout.add_child(radio_group.button(AccessScope::User, self.locale.text("for-this-user")));
        layout
            .add_child(radio_group.button(AccessScope::System, self.locale.text("for-all-users")));

        let (mut dialog, dialog_receiver) = dialog::guided_dialog(&self.locale, "", move |_| {
            Arc::unwrap_or_clone(radio_group.selection())
        });
        dialog.set_content(layout.scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn prompt_modify_search_path(&self) -> Result<GuidedDialogButton<bool>, InstallerError> {
        let mut layout = LinearLayout::vertical();
        layout.add_child(TextView::new(self.locale.text("modify-search-path-prompt")));

        let mut radio_group = RadioGroup::new();
        layout.add_child(radio_group.button(true, self.locale.text("modify-search-path")));
        layout.add_child(radio_group.button(false, self.locale.text("do-not-modify-search-path")));

        let (mut dialog, dialog_receiver) = dialog::guided_dialog(&self.locale, "", move |_| {
            Arc::unwrap_or_clone(radio_group.selection())
        });
        dialog.set_content(layout.scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn prompt_uninstall_existing(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        let (mut dialog, dialog_receiver) = dialog::guided_dialog(&self.locale, "", move |_| ());
        dialog.set_content(
            TextView::new(self.locale.text("removing-existing-before-install")).scrollable(),
        );

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn prompt_install_confirm(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        let (mut dialog, dialog_receiver) = dialog::guided_dialog(&self.locale, "", move |_| ());
        dialog.set_content(TextView::new(self.locale.text("installer-confirm")).scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn show_install_progress_dialog(&self) -> Result<(), InstallerError> {
        let dialog = dialog::progress_dialog("");

        let text = self.locale.text("installing");

        self.in_cursive(move |cursive| {
            cursive.add_layer(dialog);
            dialog::set_progress_dialog_text(cursive, &text);
        })
    }

    pub fn hide_install_progress_dialog(&self) -> Result<(), InstallerError> {
        self.in_cursive(|cursive| {
            dialog::dismiss_progress_dialog(cursive);
        })
    }

    pub fn update_install_progress(
        &mut self,
        current: u64,
        total: u64,
    ) -> Result<(), InstallerError> {
        self.in_cursive(move |cursive| {
            dialog::update_progress_dialog_bar(cursive, current, total);
        })
    }

    pub fn uninstallation_intro(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        let args = [
            ("app_name", (&self.app_name).into()),
            ("app_version", (&self.app_version).into()),
        ];
        let text = self.locale.text_args("uninstaller-intro", args);

        let (mut dialog, dialog_receiver) = dialog::guided_dialog(&self.locale, "", |_| ());
        dialog.set_content(TextView::new(text).scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn uninstallation_conclusion(&self) -> Result<(), InstallerError> {
        let args = [("app_name", (&self.app_name).into())];
        let text = self.locale.text_args("uninstaller-conclusion", args);

        let (mut dialog, dialog_receiver) = dialog::info_dialog(&self.locale, "");
        dialog.set_content(TextView::new(text).scrollable());

        self.show_wait_dialog(dialog, dialog_receiver)
    }

    pub fn show_uninstall_progress_dialog(&self) -> Result<(), InstallerError> {
        let dialog = dialog::progress_dialog("");
        let text = self.locale.text("uninstalling");

        self.in_cursive(move |cursive| {
            cursive.add_layer(dialog);

            dialog::set_progress_dialog_text(cursive, &text);
        })
    }

    pub fn hide_uninstall_progress_dialog(&self) -> Result<(), InstallerError> {
        self.in_cursive(|cursive| {
            dialog::dismiss_progress_dialog(cursive);
        })
    }

    pub fn update_uninstall_progress(
        &mut self,
        current: u64,
        total: u64,
    ) -> Result<(), InstallerError> {
        self.in_cursive(move |cursive| {
            dialog::update_progress_dialog_bar(cursive, current, total);
        })
    }

    fn in_cursive<F, T>(&self, func: F) -> Result<T, InstallerError>
    where
        F: FnOnce(&mut Cursive) -> T + Send + 'static,
        T: Send + 'static,
    {
        let channel = self.channel.as_ref().expect("TUI not running");
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);

        let result = channel.send(Box::new(move |cursive| {
            sender.send(func(cursive)).unwrap();
        }));

        if result.is_err() {
            Err(InstallerErrorKind::Terminal.into())
        } else {
            receiver
                .recv()
                .map_err(|_e| InstallerErrorKind::Terminal.into())
        }
    }
}

impl Debug for Tui {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tui").field("...", &"...").finish()
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if let Some(channel) = &mut self.channel {
            let _ = channel.send(Box::new(|cursive| cursive.quit()));
        }
    }
}
