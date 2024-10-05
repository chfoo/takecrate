use std::sync::mpsc::Receiver;

use cursive::{
    align::HAlign,
    view::Nameable,
    views::{Dialog, DialogFocus, LinearLayout, ProgressBar, TextView},
    Cursive,
};

use crate::{
    error::{InstallerError, InstallerErrorKind},
    locale::Locale,
};

pub enum GuidedDialogButton<T> {
    Exit,
    Next(T),
}

impl<T> GuidedDialogButton<T> {
    pub fn unwrap_button(self) -> Result<T, InstallerError> {
        match self {
            GuidedDialogButton::Exit => Err(InstallerErrorKind::InterruptedByUser.into()),
            GuidedDialogButton::Next(value) => Ok(value),
        }
    }
}

pub fn guided_dialog<T, F>(
    locale: &Locale,
    title: &str,
    value_callback: F,
) -> (Dialog, Receiver<GuidedDialogButton<T>>)
where
    F: Fn(&mut Cursive) -> T + Send + Sync + 'static,
    T: Send + 'static,
{
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let sender2 = sender.clone();

    let exit_text = locale.text("button-exit");
    let next_text = locale.text("button-next");

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

    (dialog, receiver)
}

pub fn info_dialog(locale: &Locale, title: &str) -> (Dialog, Receiver<()>) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);

    let button_text = locale.text("button-ok");

    let dialog = Dialog::new()
        .title(title)
        .button(button_text, move |cursive| {
            cursive.pop_layer();
            sender.send(()).unwrap();
        })
        .h_align(HAlign::Center);

    (dialog, receiver)
}

const PROGRESS_DIALOG: &str = "progress_dialog";
const PROGRESS_DIALOG_TEXT: &str = "progress_dialog_text";
const PROGRESS_DIALOG_SUBTEXT: &str = "progress_dialog_subtext";
const PROGRESS_DIALOG_PROGRESS_BAR: &str = "progress_dialog_progress_bar";

pub fn progress_dialog(title: &str) -> Dialog {
    let layout = LinearLayout::vertical()
        .child(TextView::empty().with_name(PROGRESS_DIALOG_TEXT))
        .child(TextView::empty().with_name(PROGRESS_DIALOG_SUBTEXT))
        .child(ProgressBar::new().with_name(PROGRESS_DIALOG_PROGRESS_BAR));

    Dialog::new().title(title).content(layout)
}

pub fn set_progress_dialog_text(cursive: &mut Cursive, value: &str) {
    if let Some(mut text_view) = cursive.find_name::<TextView>(PROGRESS_DIALOG_TEXT) {
        text_view.set_content(value);
    }
}

pub fn update_progress_dialog_bar(cursive: &mut Cursive, current: u64, total: u64) {
    if let Some(mut progress_bar) = cursive.find_name::<ProgressBar>(PROGRESS_DIALOG_PROGRESS_BAR) {
        progress_bar.set_max(total as usize);
        progress_bar.set_value(current as usize);
    }
}

pub fn dismiss_progress_dialog(cursive: &mut Cursive) {
    if let Some(position) = cursive.screen_mut().find_layer_from_name(PROGRESS_DIALOG) {
        cursive.screen_mut().remove_layer(position);
    }
}
