use std::{
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use windows_registry::Key;

use crate::manifest::AppId;

use super::{AccessScope, OsError};

// Notes on environment variables:
// https://winreg-kb.readthedocs.io/en/latest/sources/system-keys/Environment-variables.html
// https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables
// https://learn.microsoft.com/en-us/windows/deployment/usmt/usmt-recognized-environment-variables
// https://gist.github.com/pkfrom/f6eb82316b725a51f357
//
// Notes on app paths:
// https://learn.microsoft.com/en-us/windows/win32/shell/app-registration
//
// Notes on installed app entries:
// https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key
//
// Notes on app menu entries:
// https://superuser.com/a/960566
//
// Note on registry API:
// * open() is open read-only
// * create() is open read/write

pub const REGISTRY_ENV_USER_KEY: &str = "Environment";
pub const REGISTRY_ENV_SYSTEM_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
pub const REGISTRY_APP_PATHS_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
pub const REGISTRY_UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

fn get_registry_predefined_key(access_scope: AccessScope) -> &'static Key {
    match access_scope {
        AccessScope::User => windows_registry::CURRENT_USER,
        AccessScope::System => windows_registry::LOCAL_MACHINE,
    }
}

pub fn add_path_env_var(access_scope: AccessScope, exe_dir: &OsStr) -> Result<(), OsError> {
    // Remove any existing duplicates of exe_dir
    remove_path_env_var(access_scope, exe_dir)?;

    let predef_key = crate::os::windows::get_registry_predefined_key(access_scope);
    let key_path = match access_scope {
        AccessScope::User => REGISTRY_ENV_USER_KEY,
        AccessScope::System => REGISTRY_ENV_SYSTEM_KEY,
    };

    tracing::debug!(key_path, "opening path key read/write");
    let hkey = predef_key.create(key_path)?;

    let mut value = hkey.get_hstring("Path")?.to_os_string();
    value.push(";");
    value.push(exe_dir);

    tracing::debug!(key_path, ?value, "saving path key");
    hkey.set_expand_hstring("Path", &value.into())?;

    Ok(())
}

pub fn remove_path_env_var(access_scope: AccessScope, exe_dir: &OsStr) -> Result<(), OsError> {
    let predef_key = crate::os::windows::get_registry_predefined_key(access_scope);
    let key_path = match access_scope {
        AccessScope::User => REGISTRY_ENV_USER_KEY,
        AccessScope::System => REGISTRY_ENV_SYSTEM_KEY,
    };

    tracing::debug!(key_path, "opening path key read/write");
    let hkey = predef_key.create(key_path)?;

    let value = hkey.get_hstring("Path")?.to_os_string();
    let value = remove_part_in_path_env_var_str(&value, exe_dir);

    tracing::debug!(key_path, ?value, "saving path key");
    hkey.set_expand_hstring("Path", &value.into())?;

    Ok(())
}

fn remove_part_in_path_env_var_str(path_env_var: &OsStr, path_dir: &OsStr) -> OsString {
    let values = Vec::from_iter(
        path_env_var
            .as_encoded_bytes()
            .split(|&value| value == b';')
            .filter(|&part| {
                !part.is_empty() && !part.eq_ignore_ascii_case(path_dir.as_encoded_bytes())
            }),
    );

    unsafe {
        // SAFETY: OsString is pseudo UTF-8 and ';' is both a 1-byte code unit
        // and code point, so we are splitting and joining at a safe byte.
        OsString::from_encoded_bytes_unchecked(values.join(&b';'))
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppPathConfig {
    pub additional_path_envs: Vec<OsString>,
}

pub fn add_app_path(
    access_scope: AccessScope,
    exe_name: &str,
    exe_path: &OsStr,
    config: &AppPathConfig,
) -> Result<(), OsError> {
    let predef_key = crate::os::windows::get_registry_predefined_key(access_scope);
    let key_path = format!(
        r"{}\{}",
        crate::os::windows::REGISTRY_APP_PATHS_KEY,
        exe_name
    );

    tracing::debug!(?access_scope, key_path, "opening key read/write");
    let hkey = predef_key.create(&key_path)?;

    tracing::debug!(?access_scope, key_path, ?exe_path, "setting key");
    hkey.set_hstring("", &exe_path.into())?;

    if !config.additional_path_envs.is_empty() {
        let value = config.additional_path_envs.join(OsStr::new(";"));

        hkey.set_expand_hstring("Path", &value.into())?;
    }

    Ok(())
}

pub fn remove_app_path(access_scope: AccessScope, exe_name: &str) -> Result<(), OsError> {
    let predef_key = crate::os::windows::get_registry_predefined_key(access_scope);
    let key_path = format!(
        r"{}\{}",
        crate::os::windows::REGISTRY_APP_PATHS_KEY,
        exe_name
    );

    tracing::debug!(?access_scope, key_path, "deleting key tree");
    if predef_key.get_type(&key_path).is_ok() {
        predef_key.remove_tree(key_path)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct UninstallEntryConfig {
    pub manifest_path: PathBuf,
    pub display_name: String,
    pub display_version: String,
    pub publisher: String,
    pub estimated_size: u64,
    pub quiet_exe_args: OsString,
}

pub fn add_uninstall_entry(
    access_scope: AccessScope,
    app_id: &AppId,
    exe_path: &OsStr,
    exe_args: &OsStr,
    config: &UninstallEntryConfig,
) -> Result<(), OsError> {
    let predef_key = crate::os::windows::get_registry_predefined_key(access_scope);
    let key_path = format!(r"{}\{}", REGISTRY_UNINSTALL_KEY, app_id.uuid());

    tracing::debug!(?access_scope, key_path, "creating key");
    let hkey = predef_key.create(key_path)?;

    let mut uninstall_string = OsString::new();
    uninstall_string.push("\"");
    uninstall_string.push(exe_path);
    uninstall_string.push("\" ");
    uninstall_string.push(exe_args);

    tracing::debug!(?uninstall_string, "writing keys");
    hkey.set_hstring("UninstallString", &uninstall_string.into())?;
    hkey.set_string("DisplayName", &config.display_name)?;
    hkey.set_hstring(
        "takecrate_manifest_path",
        &config.manifest_path.as_os_str().into(),
    )?;

    if !config.display_version.is_empty() {
        hkey.set_string("DisplayVersion", &config.display_version)?;
    }

    if !config.publisher.is_empty() {
        hkey.set_string("Publisher", &config.publisher)?;
    }

    if config.estimated_size > 0 {
        // It is in kilobytes
        hkey.set_u32(
            "EstimatedSize",
            (config.estimated_size >> 10).try_into().unwrap_or(u32::MAX),
        )?;
    }

    if !config.quiet_exe_args.is_empty() {
        let mut quiet_string = OsString::new();
        quiet_string.push("\"");
        quiet_string.push(exe_path);
        quiet_string.push("\" ");
        quiet_string.push(&config.quiet_exe_args);

        hkey.set_hstring("QuietInstallString", &quiet_string.into())?;
    }

    Ok(())
}

pub fn remove_uninstall_entry(access_scope: AccessScope, app_id: &AppId) -> Result<(), OsError> {
    let predef_key = crate::os::windows::get_registry_predefined_key(access_scope);
    let key_path = format!(r"{}\{}", REGISTRY_UNINSTALL_KEY, app_id.uuid());

    tracing::debug!(?access_scope, key_path, "removing key tree");

    if predef_key.open(&key_path).is_ok() {
        predef_key.remove_tree(key_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_in_path_env_var() {
        assert_eq!(
            remove_part_in_path_env_var_str(
                OsStr::new(r"C:\things\bin;C:\Rust\bin;C:\Windows Apps"),
                OsStr::new(r"c:\rust\bin")
            ),
            r"C:\things\bin;C:\Windows Apps",
        )
    }
}
