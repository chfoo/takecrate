use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, LazyLock, Mutex},
};

use fluent_bundle::FluentValue;
use fluent_templates::{ArcLoader, LanguageIdentifier, Loader};

#[cfg(feature = "i18n-static")]
fluent_templates::static_loader! {
    static LOCALES = {
        locales: "locales",
        fallback_language: "en-US",
        core_locales: "locales/core.ftl",
    };
}

static CUSTOM_LOADER: Mutex<Option<Arc<ArcLoader>>> = Mutex::new(None);

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

#[cfg(feature = "i18n-custom")]
pub fn set_custom_loader(loader: ArcLoader) {
    let mut guard = CUSTOM_LOADER.lock().unwrap();
    guard.replace(Arc::new(loader));
}

pub struct Locale {
    lang_id: LanguageIdentifier,
    custom_loader: Option<Arc<ArcLoader>>,
}

impl Locale {
    fn new(id: LanguageIdentifier) -> Self {
        Self {
            lang_id: id,
            custom_loader: CUSTOM_LOADER.lock().unwrap().clone(),
        }
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
        if let Some(loader) = &self.custom_loader {
            loader.lookup(&self.lang_id, text_id)
        } else {
            #[cfg(feature = "i18n-static")]
            {
                LOCALES.lookup(&self.lang_id, text_id)
            }
            #[cfg(not(feature = "i18n-static"))]
            {
                text_id.to_string()
            }
        }
    }

    pub fn text_args<'a, A>(&self, text_id: &str, args: A) -> String
    where
        A: Into<HashMap<&'a str, FluentValue<'a>>>,
    {
        let args: HashMap<&str, FluentValue<'_>> = args.into();

        if let Some(loader) = &self.custom_loader {
            loader.lookup_with_args(&self.lang_id, text_id, &args)
        } else {
            #[cfg(feature = "i18n-static")]
            {
                LOCALES.lookup_with_args(&self.lang_id, text_id, &args)
            }
            #[cfg(not(feature = "i18n-static"))]
            {
                text_id.to_string()
            }
        }
    }
}
