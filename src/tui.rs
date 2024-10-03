use std::{
    fmt::Debug,
    sync::{mpsc::SyncSender, Arc},
    thread::JoinHandle,
};

use cursive::{
    align::HAlign,
    view::{Nameable, Scrollable},
    views::{Dialog, DialogFocus, LinearLayout, ProgressBar, RadioGroup, TextView},
    CbSink, Cursive, CursiveExt,
};

use crate::{
    error::{InstallerError, InstallerErrorKind},
    os::AccessScope,
};

pub enum GuidedDialogButton<T> {
    Exit,
    Next(T),
}

pub struct Tui {
    channel: Option<CbSink>,
    handle: Option<JoinHandle<std::io::Result<()>>>,
    app_name: String,
    app_version: String,
}

impl Tui {
    pub fn new() -> Self {
        Self {
            channel: None,
            handle: None,
            app_name: String::new(),
            app_version: String::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.channel.is_some()
    }

    pub fn set_name(&mut self, app_name: &str, app_version: &str) {
        self.app_name = app_name.to_string();
        self.app_version = app_version.to_string();
    }

    pub fn run_background(&mut self) {
        assert!(self.channel.is_none());

        let (sender, receiver) = std::sync::mpsc::sync_channel(1);

        let join_handle = std::thread::spawn(move || {
            let mut cursive = cursive::Cursive::new();

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

    pub fn show_error<E>(&self, error: E) -> Result<(), InstallerError>
    where
        E: std::error::Error,
    {
        let label = crate::locale::text("error-occurred");
        let details_label = crate::locale::text("error-details");
        let error_message = crate::error::format_error(error);

        let layout = LinearLayout::vertical()
            .child(TextView::new(label))
            .child(TextView::new("\n"))
            .child(TextView::new(details_label))
            .child(TextView::new("\n"))
            .child(TextView::new(error_message).scrollable());

        self.callback(|cursive, sender| {
            let dialog = make_info_dialog("", sender).content(layout);

            cursive.add_layer(dialog);
        })
    }

    pub fn show_unneeded_install(&self, is_uninstall: bool) -> Result<(), InstallerError> {
        let title = crate::locale::text("error-occurred");
        let text = if is_uninstall {
            crate::locale::text("app-not-installed")
        } else {
            crate::locale::text("app-already-installed")
        };

        self.callback(move |cursive, sender| {
            let dialog = make_info_dialog(&title, sender).content(TextView::new(text).scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn installation_intro(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        let title = crate::locale::text("installer-title");
        let args = [
            ("app_name", (&self.app_name).into()),
            ("app_version", (&self.app_version).into()),
        ];
        let text = crate::locale::text_args("installer-intro", args);

        self.callback(move |cursive, sender| {
            let dialog = make_guided_dialog(&title, sender, |_| ())
                .content(TextView::new(text).scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn installation_conclusion(&self) -> Result<(), InstallerError> {
        let title = crate::locale::text("installer-title");
        let args = [("app_name", (&self.app_name).into())];
        let text = crate::locale::text_args("installer-conclusion", args);

        self.callback(move |cursive, sender| {
            let dialog = make_info_dialog(&title, sender).content(TextView::new(text).scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn prompt_access_scope(&self) -> Result<GuidedDialogButton<AccessScope>, InstallerError> {
        self.callback(|cursive, sender| {
            let title = crate::locale::text("installer-title");
            let mut layout = LinearLayout::vertical();
            layout.add_child(TextView::new(crate::locale::text("access-scope-prompt")));

            let mut radio_group = RadioGroup::new();
            layout.add_child(
                radio_group.button(AccessScope::User, crate::locale::text("for-this-user")),
            );
            layout.add_child(
                radio_group.button(AccessScope::System, crate::locale::text("for-all-users")),
            );

            let dialog = make_guided_dialog(&title, sender, move |_| {
                Arc::unwrap_or_clone(radio_group.selection())
            })
            .content(layout.scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn prompt_modify_search_path(&self) -> Result<GuidedDialogButton<bool>, InstallerError> {
        self.callback(|cursive, sender| {
            let title = crate::locale::text("installer-title");
            let mut layout = LinearLayout::vertical();
            layout.add_child(TextView::new(crate::locale::text(
                "modify-search-path-prompt",
            )));

            let mut radio_group = RadioGroup::new();
            layout.add_child(radio_group.button(true, crate::locale::text("modify-search-path")));
            layout.add_child(
                radio_group.button(false, crate::locale::text("do-not-modify-search-path")),
            );

            let dialog = make_guided_dialog(&title, sender, move |_| {
                Arc::unwrap_or_clone(radio_group.selection())
            })
            .content(layout.scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn prompt_install_confirm(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        self.callback(|cursive, sender| {
            let title = crate::locale::text("installer-title");
            let dialog = make_guided_dialog(&title, sender, move |_| ())
                .content(TextView::new(crate::locale::text("installer-confirm")).scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn show_install_progress_dialog(&self) -> Result<(), InstallerError> {
        self.callback(|cursive, sender| {
            let title = crate::locale::text("installer-title");
            cursive.add_layer(make_progress_dialog(&title));

            let mut text_view = cursive.find_name::<TextView>(PROGRESS_DIALOG_TEXT).unwrap();
            text_view.set_content(crate::locale::text("installing"));

            sender.send(()).unwrap();
        })
    }

    pub fn hide_install_progress_dialog(&self) -> Result<(), InstallerError> {
        self.callback(|cursive, sender| {
            if let Some(position) = cursive.screen_mut().find_layer_from_name(PROGRESS_DIALOG) {
                cursive.screen_mut().remove_layer(position);
            }

            sender.send(()).unwrap();
        })
    }

    pub fn update_install_progress(
        &mut self,
        current: u64,
        total: u64,
    ) -> Result<(), InstallerError> {
        self.callback(move |cursive, sender| {
            let mut progress_bar = cursive
                .find_name::<ProgressBar>(PROGRESS_DIALOG_PROGRESS_BAR)
                .unwrap();
            progress_bar.set_max(total as usize);
            progress_bar.set_value(current as usize);
            sender.send(()).unwrap();
        })
    }

    pub fn uninstallation_intro(&self) -> Result<GuidedDialogButton<()>, InstallerError> {
        let args = [
            ("app_name", (&self.app_name).into()),
            ("app_version", (&self.app_version).into()),
        ];
        let text = crate::locale::text_args("uninstaller-intro", args);

        self.callback(|cursive, sender| {
            let dialog =
                make_guided_dialog(&crate::locale::text("uninstaller-title"), sender, |_| ())
                    .content(TextView::new(text).scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn uninstallation_conclusion(&self) -> Result<(), InstallerError> {
        let args = [("app_name", (&self.app_name).into())];
        let text = crate::locale::text_args("uninstaller-conclusion", args);

        self.callback(|cursive, sender| {
            let dialog = make_info_dialog(&crate::locale::text("uninstaller-title"), sender)
                .content(TextView::new(text).scrollable());

            cursive.add_layer(dialog);
        })
    }

    pub fn show_uninstall_progress_dialog(&self) -> Result<(), InstallerError> {
        self.callback(|cursive, sender| {
            let title = crate::locale::text("uninstaller-title");
            cursive.add_layer(make_progress_dialog(&title));

            let mut text_view = cursive.find_name::<TextView>(PROGRESS_DIALOG_TEXT).unwrap();
            text_view.set_content(crate::locale::text("uninstalling"));

            sender.send(()).unwrap();
        })
    }

    pub fn hide_uninstall_progress_dialog(&self) -> Result<(), InstallerError> {
        self.callback(|cursive, sender| {
            if let Some(position) = cursive.screen_mut().find_layer_from_name(PROGRESS_DIALOG) {
                cursive.screen_mut().remove_layer(position);
            }

            sender.send(()).unwrap();
        })
    }

    pub fn update_uninstall_progress(
        &mut self,
        current: u64,
        total: u64,
    ) -> Result<(), InstallerError> {
        self.callback(move |cursive, sender| {
            let mut progress_bar = cursive
                .find_name::<ProgressBar>(PROGRESS_DIALOG_PROGRESS_BAR)
                .unwrap();
            progress_bar.set_max(total as usize);
            progress_bar.set_value(current as usize);
            sender.send(()).unwrap();
        })
    }

    fn callback<F, T>(&self, func: F) -> Result<T, InstallerError>
    where
        F: FnOnce(&mut Cursive, SyncSender<T>) + Send + 'static,
        T: Send + 'static,
    {
        let channel = self.channel.as_ref().expect("TUI not running");
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);

        let result = channel.send(Box::new(move |cursive| {
            func(cursive, sender);
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

fn make_guided_dialog<T, F>(
    title: &str,
    sender: SyncSender<GuidedDialogButton<T>>,
    value_callback: F,
) -> Dialog
where
    F: Fn(&mut Cursive) -> T + Send + Sync + 'static,
    T: Send + 'static,
{
    let sender2 = sender.clone();
    let exit_text = crate::locale::text("button-exit");
    let next_text = crate::locale::text("button-next");

    let mut dialog = Dialog::new().title(title).h_align(HAlign::Right);

    dialog.add_button(exit_text, move |cursive| {
        cursive.pop_layer();
        sender.send(GuidedDialogButton::Exit).unwrap();
    });
    dialog.add_button(next_text, move |cursive| {
        cursive.pop_layer();
        let value = value_callback(cursive);
        sender2.send(GuidedDialogButton::Next(value)).unwrap();
    });

    let _ = dialog.set_focus(DialogFocus::Button(1));

    dialog
}

fn make_info_dialog(title: &str, sender: SyncSender<()>) -> Dialog {
    let button_text = crate::locale::text("button-ok");

    Dialog::new()
        .title(title)
        .button(button_text, move |cursive| {
            cursive.pop_layer();
            sender.send(()).unwrap();
        })
        .h_align(HAlign::Center)
}

const PROGRESS_DIALOG: &str = "progress_dialog";
const PROGRESS_DIALOG_TEXT: &str = "progress_dialog_text";
const PROGRESS_DIALOG_SUBTEXT: &str = "progress_dialog_subtext";
const PROGRESS_DIALOG_PROGRESS_BAR: &str = "progress_dialog_progress_bar";

fn make_progress_dialog(title: &str) -> Dialog {
    let layout = LinearLayout::vertical()
        .child(TextView::empty().with_name(PROGRESS_DIALOG_TEXT))
        .child(TextView::empty().with_name(PROGRESS_DIALOG_SUBTEXT))
        .child(ProgressBar::new().with_name(PROGRESS_DIALOG_PROGRESS_BAR));

    Dialog::new().title(title).content(layout)
}
