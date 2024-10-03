use std::path::{Path, PathBuf};

use crate::{
    error::{InstallerError, InstallerErrorKind},
    os::AccessScope,
};

use super::{AppId, DiskManifest};

/// Returns the expected file path of the [`DiskManifest`] on the user's machine.
pub fn manifest_path(app_id: &AppId, access_scope: AccessScope) -> Result<PathBuf, InstallerError> {
    let state_path = match std::env::consts::FAMILY {
        "windows" => match access_scope {
            AccessScope::User => PathBuf::from(crate::os::env_var("LocalAppData")?),
            AccessScope::System => PathBuf::from(crate::os::env_var("ProgramData")?),
        },

        "unix" => match access_scope {
            AccessScope::User => {
                if let Some(value) = std::env::var_os("XDG_CONFIG_HOME") {
                    PathBuf::from(value)
                } else {
                    PathBuf::from(crate::os::env_var("HOME")?).join(".config")
                }
            }
            AccessScope::System => PathBuf::from("/var/local/lib"),
        },
        _ => return Err(InstallerErrorKind::UnsupportedOsFamily.into()),
    }
    .join("io.crates.takecrate");

    let manifest_path = state_path.join(format!(
        "takecrate-manifest__{}.ron",
        app_id.namespaced_id()
    ));

    Ok(manifest_path)
}

/// Finds the [`DiskManifest`] on the machine and returns it.
///
/// If it is not found, an error kind [`InstallerErrorKind::DiskManifestNotFound`] is returned.
///
/// Internal implementation detail: Attempts to search user, then system.
pub fn discover_manifest(exe_path: &Path, app_id: &AppId) -> Result<DiskManifest, InstallerError> {
    let single_dir_path = exe_path.join(format!(
        "../takecrate-manifest__{}.ron",
        app_id.namespaced_id()
    ));

    if single_dir_path.exists() {
        return DiskManifest::load(&single_dir_path);
    }

    let user_path = manifest_path(app_id, AccessScope::User)?;

    if user_path.exists() {
        return DiskManifest::load(&user_path);
    }

    let system_path = manifest_path(app_id, AccessScope::System)?;

    if system_path.exists() {
        return DiskManifest::load(&system_path);
    }

    Err(InstallerErrorKind::DiskManifestNotFound.into())
}
