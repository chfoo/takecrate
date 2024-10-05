use std::{collections::HashMap, str::FromStr, sync::LazyLock};

use fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader};

fluent_templates::static_loader! {
    static LOCALES = {
        locales: "locales",
        fallback_language: "en-US",
    };
}

fn current_lang_id() -> &'static LanguageIdentifier {
    static LANG_ID: LazyLock<LanguageIdentifier> = LazyLock::new(|| {
        let locale_string = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());

        match LanguageIdentifier::from_str(&locale_string) {
            Ok(value) => value,
            Err(_) => fluent_templates::langid!("en-US"),
        }
    });

    &LANG_ID
}

pub fn current_lang_tag() -> String {
    current_lang_id().to_string()
}

pub struct Locale {
    lang_id: LanguageIdentifier,
}

impl Locale {
    fn new(id: LanguageIdentifier) -> Self {
        Self { lang_id: id }
    }

    pub fn with_system() -> Self {
        Self::new(current_lang_id().clone())
    }

    // pub fn with_language_tag(value: &str) -> Self {
    //     let lang_id = match LanguageIdentifier::from_str(value) {
    //         Ok(value) => value,
    //         Err(_) => fluent_templates::langid!("en-US"),
    //     };
    //     Self::new(lang_id)
    // }

    pub fn set_language_tag(&mut self, value: &str) {
        self.lang_id = match LanguageIdentifier::from_str(value) {
            Ok(value) => value,
            Err(_) => fluent_templates::langid!("en-US"),
        };
    }

    pub fn text(&self, text_id: &str) -> String {
        LOCALES.lookup(&self.lang_id, text_id)
    }

    pub fn text_args<'a, A>(&self, text_id: &str, args: A) -> String
    where
        A: Into<HashMap<&'a str, FluentValue<'a>>>,
    {
        let args: HashMap<&str, FluentValue<'_>> = args.into();
        LOCALES.lookup_with_args(&self.lang_id, text_id, &args)
    }
}
