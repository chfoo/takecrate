use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Reexport from [`uuid`] crate:
pub use uuid::Uuid;

/// Represents a unique ID for an application.
///
/// There are 3 representations: namespaced, plain, and UUID.
///
/// The namespaced format is based on the Java package naming convention, a
/// namespaced ID format modeled after the domain name system (with each
/// segment in reversed order). See [`validate_namespaced_id()`] for the
/// format rules.
///
/// Note that you do not need to own a domain name. The domain name registry
/// ensures a unique namespace but this is not the only method. You can use
/// namespaces such as your crate repository or email account.
///
/// Examples of namespaced IDs:
///
/// * `net.example.myapp`
/// * `io.crates.my_app`
/// * `io.github.myusername123.my-app`
///
/// The plain format is the application ID without a namespace.
///
/// The UUID format, as it implies, is the UUID for the application.
///
/// This crate will use the plain format in most cases, such as, directory
/// names. The dotted and UUID format is used internally and for the OS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppId {
    plain_id: String,
    namespaced_id: String,
    uuid: uuid::Uuid,
}

impl AppId {
    /// Creates a new struct with the given namespaced ID.
    ///
    /// The plain format ID will be derived using the last segment of the
    /// given namespaced ID.
    ///
    /// The UUID will be derived from the dotted ID.
    pub fn new(namespaced_id: &str) -> Result<Self, AppIdError> {
        validate_namespaced_id(namespaced_id)?;

        let uuid = app_id_to_uuid(namespaced_id);

        Ok(Self {
            plain_id: namespaced_id.split('.').last().unwrap().to_string(),
            namespaced_id: namespaced_id.to_string(),
            uuid,
        })
    }

    /// Override the plain ID format.
    ///
    /// No validation is performed on the value.
    pub fn with_plain_id(mut self, value: &str) -> Self {
        self.plain_id = value.to_string();
        self
    }

    /// Override the UUID.
    pub fn with_uuid(mut self, value: Uuid) -> Self {
        self.uuid = value;
        self
    }

    /// Returns the plain ID.
    pub fn plain_id(&self) -> &str {
        &self.plain_id
    }

    /// Returns the namespaced ID.
    pub fn namespaced_id(&self) -> &str {
        &self.namespaced_id
    }

    /// Returns the UUID.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

/// Returns whether the given namespaced ID is valid.
///
/// The format uses the rules:
///
/// * No longer than 100 characters in total
/// * Be at least 2 segments long
/// * Each segment must be 2 characters long
/// * Valid segment characters are letters, numbers, hyphen, and underscore
/// * A segment starts with a letter
/// * Case-insensitive (hyphen and underscore are compared equivalent as well)
///
pub fn validate_namespaced_id(value: &str) -> Result<(), AppIdError> {
    if value.len() > 100 {
        return Err(AppIdError::Length);
    }

    let segments = value.split('.').collect::<Vec<&str>>();

    if segments.len() < 2 {
        return Err(AppIdError::SegmentCount);
    }

    for segment in segments {
        if segment.len() < 2 {
            return Err(AppIdError::SegmentLength);
        }

        if !segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppIdError::Character);
        }

        if !segment.chars().next().unwrap().is_ascii_alphabetic() {
            return Err(AppIdError::FirstCharacter);
        }
    }

    Ok(())
}

/// Normalize the namespaced ID.
///
/// This converts it to lowercase and replaces all hyphens with underscores.
pub fn normalize_namespaced_id(value: &str) -> String {
    value.replace("-", "_").to_ascii_lowercase()
}

const NAMESPACE: Uuid = uuid::uuid!("0192391a-2817-7e1c-988d-5aef70264a82");

/// Returns a UUID derived from the given value.
pub fn app_id_to_uuid(value: &str) -> Uuid {
    Uuid::new_v5(&NAMESPACE, normalize_namespaced_id(value).as_bytes())
}

/// Metadata such display names and versions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AppMetadata {
    /// Name of application shown to the user.
    pub display_name: String,
    /// Version of application shown to the user.
    pub display_version: String,
    /// Localized application names.
    ///
    /// Each key is a BCP 47 language tag.
    pub locale_display_name: HashMap<String, String>,
}

impl AppMetadata {
    /// Returns a localized display name with fallback.
    pub fn get_display_name(&self, lang_tag: &str) -> &str {
        self.locale_display_name
            .get(lang_tag)
            .map(|v| v.as_str())
            .unwrap_or_else(|| self.display_name.as_str())
    }
}

/// Error for ID validation.
#[derive(Debug, thiserror::Error)]
pub enum AppIdError {
    /// Invalid character within a segment.
    #[error("character")]
    Character,

    /// Invalid first character of a segment.
    #[error(" first character")]
    FirstCharacter,

    /// Number of segments is invalid.
    #[error("segment count")]
    SegmentCount,

    /// Length of a segment is invalid.
    #[error("segment length")]
    SegmentLength,

    /// Total length of the ID is invalid.
    #[error("length")]
    Length,
}
