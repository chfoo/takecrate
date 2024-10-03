//! Common error types.
//!
//! See [`InstallerError`] and [`InstallerErrorKind`] for details.
use std::fmt::Display;

/// Main error type for this crate.
#[derive(Debug, thiserror::Error)]
pub struct InstallerError {
    kind: InstallerErrorKind,
    context: String,
    source: Option<Box<dyn std::error::Error + 'static + Send + Sync>>,
}

impl InstallerError {
    /// Creates a new error with the given error kind.
    pub fn new(kind: InstallerErrorKind) -> Self {
        Self {
            kind,
            context: String::new(),
            source: None,
        }
    }

    /// Adds a source error.
    pub fn with_source<S>(mut self, source: S) -> Self
    where
        S: std::error::Error + 'static + Send + Sync,
    {
        self.source = Some(Box::new(source));
        self
    }

    /// Adds a string with a contextual description of the error.
    pub fn with_context<C>(mut self, value: C) -> Self
    where
        C: AsRef<str>,
    {
        if !self.context.is_empty() {
            self.context.push_str(": ");
        }
        self.context.push_str(value.as_ref());
        self
    }

    /// Returns the error kind.
    pub fn kind(&self) -> &InstallerErrorKind {
        &self.kind
    }

    /// Returns the contextual description.
    pub fn context(&self) -> &str {
        &self.context
    }

    /// Returns whether the error kind is [`InstallerErrorKind::Io`].
    pub fn is_io(&self) -> bool {
        self.as_io().is_some()
    }

    /// Returns a reference to the IO error when the kind is [`InstallerErrorKind::Io`].
    pub fn as_io(&self) -> Option<&std::io::Error> {
        if self.kind.is_io() {
            if let Some(source) = &self.source {
                if let Some(error) = source.downcast_ref() {
                    return Some(error);
                }
            }
        }
        None
    }
}

impl Display for InstallerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.context.is_empty() {
            f.write_str(&self.context)?;
            f.write_str(": ")?;
        }

        self.kind.fmt(f)?;

        Ok(())
    }
}

impl From<InstallerErrorKind> for InstallerError {
    fn from(value: InstallerErrorKind) -> Self {
        Self::new(value)
    }
}

impl From<std::io::Error> for InstallerError {
    fn from(value: std::io::Error) -> Self {
        Self::new(InstallerErrorKind::Io).with_source(value)
    }
}

impl From<AdditionalContext> for InstallerError {
    fn from(value: AdditionalContext) -> Self {
        Self::new(InstallerErrorKind::Other).with_source(value)
    }
}

/// Error category for [`InstallerError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InstallerErrorKind {
    /// Input/Output error usually from [`std::io::Error`].
    #[error("input/output error")]
    Io,

    /// Invalid input or argument type to a function.
    #[error("invalid input or argument")]
    InvalidInput,

    /// Invalid data or value provided to a function.
    #[error("invalid data or value")]
    InvalidData,

    /// Unsupported OS family ("windows", "unix", etc.).
    #[error("unsupported OS family")]
    UnsupportedOsFamily,

    /// Environment variable was missing or malformed.
    #[error("invalid environment variable")]
    InvalidEnvironmentVariable,

    /// Filename of the binary could not be determined.
    #[error("unknown executable path")]
    UnknownExecutablePath,

    /// [`crate::inst::PackageManifest`] has a invalid value.
    #[error("invalid package manifest")]
    InvalidPackageManifest,

    /// Could not locate the [`crate::manifest::DiskManifest`]
    ///
    /// The binary may not be installed or it was not properly installed.
    #[error("disk manifest not found")]
    DiskManifestNotFound,

    /// [`crate::manifest::DiskManifest`] could not be parsed.
    ///
    /// It may be tampered, corrupted, or an incompatible version.
    #[error("malformed disk manifest")]
    MalformedDiskManifest,

    /// [`crate::manifest::DiskManifest`] has an invalid value.
    #[error("invalid disk manifest")]
    InvalidDiskManifest,

    /// The [`crate::manifest::DiskManifest`] installed does not match this binary.
    ///
    /// The given application ID needs to match to the one installed.
    #[error("mismatched disk manifest")]
    MismatchedDiskManifest,

    /// There was a file in the destination that does not match the expected checksum.
    #[error("unknown file in destination")]
    UnknownFileInDestination,

    /// Internal console/terminal library returned an error.
    #[error("console/terminal error")]
    Terminal,

    /// Indicates the application is (likely) installed.
    ///
    /// It may return a false positive when the install/uninstall failed.
    #[error("application is already installed")]
    AlreadyInstalled,

    /// Indicates a guided interactive session was aborted by the user.
    #[error("interrupted by user")]
    InterruptedByUser,

    /// Any other error.
    #[error("other")]
    Other,
}

impl InstallerErrorKind {
    /// Returns whether it is the Io variant.
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io)
    }
}

/// Modify `Result<T, InstallerError>` with context.
pub trait AddInstallerContext<T> {
    /// Add context using the given string when Err.
    fn inst_context<C>(self, context: C) -> Result<T, InstallerError>
    where
        C: AsRef<str>;

    /// Add context using the evaluated function when Err.
    fn inst_contextc<C, CT>(self, context: C) -> Result<T, InstallerError>
    where
        C: FnOnce() -> CT,
        CT: AsRef<str>;
}

impl<T> AddInstallerContext<T> for Result<T, InstallerError> {
    fn inst_context<C>(self, context: C) -> Result<T, InstallerError>
    where
        C: AsRef<str>,
    {
        self.map_err(|error| error.with_context(context.as_ref()))
    }

    fn inst_contextc<C, CT>(self, context: C) -> Result<T, InstallerError>
    where
        C: FnOnce() -> CT,
        CT: AsRef<str>,
    {
        self.map_err(|error| error.with_context(context().as_ref()))
    }
}

/// Contains a contextual description of an error.
///
/// This isn't a real error, but allows injecting context in the error stack.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AdditionalContext {
    message: String,
    #[source]
    source: Box<dyn std::error::Error + Sync + Send + 'static>,
}

impl AdditionalContext {
    /// Creates a new context error with the given message and source error.
    pub fn new<E>(message: String, source: E) -> Self
    where
        E: std::error::Error + Sync + Send + 'static,
    {
        Self {
            message,
            source: Box::new(source),
        }
    }
}

/// Trait for wrapping errors in Result with descriptive context strings.
pub trait AddContext<T, E, A> {
    /// Map the error with an error containing the context string.
    fn with_context<C>(self, context: C) -> Result<T, A>
    where
        C: Into<String>;

    /// Map the error with an error containing the context string evaluated from a function.
    fn with_contextc<C, CT>(self, context: C) -> Result<T, A>
    where
        C: FnOnce(&E) -> CT,
        CT: Into<String>;
}

impl<T, E> AddContext<T, E, AdditionalContext> for Result<T, E>
where
    E: std::error::Error + Sync + Send + 'static,
{
    fn with_context<C>(self, context: C) -> Result<T, AdditionalContext>
    where
        C: Into<String>,
    {
        self.map_err(|error| AdditionalContext::new(context.into(), error))
    }

    fn with_contextc<C, CT>(self, context: C) -> Result<T, AdditionalContext>
    where
        C: FnOnce(&E) -> CT,
        CT: Into<String>,
    {
        self.map_err(|error| AdditionalContext::new(context(&error).into(), error))
    }
}

pub(crate) fn format_error<E>(error: E) -> String
where
    E: std::error::Error,
{
    let mut buf = error.to_string();

    let mut error: Box<&dyn std::error::Error> = Box::new(&error);

    while let Some(source) = error.source() {
        error = Box::new(source);
        buf.push_str("\n↳");
        buf.push_str(&error.to_string());
    }

    buf
}
