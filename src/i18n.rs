//! Internationalization and localization (globalization) support.

#[cfg(feature = "i18n-custom")]
pub use fluent_bundle;
#[cfg(feature = "i18n-custom")]
pub use fluent_templates;

#[cfg(feature = "i18n-custom")]
/// Sets a global language translation loader.
pub fn set_custom_loader(loader: fluent_templates::ArcLoader) {
    crate::locale::set_custom_loader(loader);
}
