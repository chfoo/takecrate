// Notes for app menus:
// https://specifications.freedesktop.org/desktop-entry-spec/latest/index.html
//
// Notes on filesystem directories:
// https://specifications.freedesktop.org/basedir-spec/latest/index.html
// https://en.wikipedia.org/wiki/Filesystem_Hierarchy_Standard

use std::ffi::OsStr;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{fs::File, path::Path, sync::LazyLock};

use crate::manifest::FileType;

use super::{AccessScope, OsError};

pub fn get_umask() -> u32 {
    static UMASK: LazyLock<u32> = LazyLock::new(|| {
        let value = unsafe {
            // SAFETY: we're only getting and setting integers.
            let current = libc::umask(0o022);
            libc::umask(current);
            current
        };
        // Cast used because it might be u16 on macOS.
        value as u32
    });

    *UMASK
}

pub fn get_effective_posix_permission(file_type: FileType) -> u32 {
    let full = match file_type {
        FileType::Executable => 0o777,
        _ => 0o666,
    };
    full & !get_umask()
}

pub fn set_posix_permission(target: &Path, mode: u32) -> std::io::Result<()> {
    let mut perm = target.metadata()?.permissions();
    perm.set_mode(mode);
    std::fs::set_permissions(target, perm)?;
    Ok(())
}

const PROFILE_SHELL_TEMPLATE_SNIPPET: &str = r#"
## <io.crates.takecrate> Automatically inserted snippet
if [ -d "{path}" ] ; then
    PATH="{path}:$PATH"
fi
## </io.crates.takecrate>
"#;

pub fn add_path_env_var(
    access_scope: AccessScope,
    exe_dir: &OsStr,
    profile: &Path,
) -> Result<(), OsError> {
    match access_scope {
        AccessScope::User => add_path_env_var_user(exe_dir, profile),
        AccessScope::System => unimplemented!(),
    }
}

pub fn remove_path_env_var(
    access_scope: AccessScope,
    exe_dir: &OsStr,
    profile: &Path,
) -> Result<(), OsError> {
    match access_scope {
        AccessScope::User => remove_path_env_var_user(exe_dir, profile),
        AccessScope::System => unimplemented!(),
    }
}

fn add_path_env_var_user(exe_dir: &OsStr, profile_path: &Path) -> Result<(), OsError> {
    let home = get_home()?;

    let exe_dir_shell_path = path_to_shell_script_path(Path::new(exe_dir), &home);
    verify_safe_for_shell_script(&exe_dir_shell_path)?;

    if profile_path.exists() {
        tracing::debug!(?profile_path, "reading profile");
        let contents = std::fs::read_to_string(profile_path)?;

        if contents.contains(&exe_dir_shell_path) {
            return Ok(());
        }
    }

    let snippet = PROFILE_SHELL_TEMPLATE_SNIPPET.replace("{path}", &exe_dir_shell_path);

    tracing::debug!(snippet, "saving profile");
    let mut file = File::options()
        .create(true)
        .append(true)
        .open(profile_path)?;
    file.write_all(snippet.as_bytes())?;
    file.flush()?;

    Ok(())
}

fn remove_path_env_var_user(exe_dir: &OsStr, profile_path: &Path) -> Result<(), OsError> {
    let home = get_home()?;

    let exe_dir_shell_path = path_to_shell_script_path(Path::new(exe_dir), &home);
    verify_safe_for_shell_script(&exe_dir_shell_path)?;

    if !profile_path.exists() {
        return Ok(());
    }

    let snippet = PROFILE_SHELL_TEMPLATE_SNIPPET.replace("{path}", &exe_dir_shell_path);

    tracing::debug!(?profile_path, "reading profile");
    let contents = std::fs::read_to_string(profile_path)?;

    if !contents.contains(&snippet) {
        return Ok(());
    }

    let new_contents = contents.replace(&snippet, "");

    tracing::debug!(?profile_path, "saving profile");
    std::fs::write(profile_path, new_contents)?;

    Ok(())
}

pub fn get_home() -> Result<PathBuf, OsError> {
    let home = std::env::var_os("HOME").ok_or(OsError::Other("missing HOME"))?;
    Ok(PathBuf::from(home))
}

pub fn get_current_shell_profile() -> Result<PathBuf, OsError> {
    let home = get_home()?;
    let zsh_profile = home.join(".zprofile");
    let bash_profile = home.join(".bash_profile");
    let default_profile = home.join(".profile");

    let shell_path = std::env::var("SHELL").unwrap_or_default();
    let shell_path = PathBuf::from(shell_path);

    if let Some(shell_name) = shell_path.file_name() {
        let shell_name = shell_name.to_str().unwrap_or_default();

        match shell_name {
            "zsh" if zsh_profile.exists() => return Ok(zsh_profile),
            "bash" if bash_profile.exists() => return Ok(bash_profile),
            _ => {}
        }

        if default_profile.exists() {
            return Ok(default_profile);
        }

        match shell_name {
            "zsh" => return Ok(zsh_profile),
            "bash" => return Ok(bash_profile),
            _ => {}
        }
    }

    Ok(default_profile)
}

fn verify_safe_for_shell_script(path_str: &str) -> Result<(), OsError> {
    if path_str.chars().any(|c| c.is_control() || c == '"') {
        return Err(OsError::Other("invalid path character"));
    }

    Ok(())
}

fn path_to_shell_script_path(path: &Path, home: &Path) -> String {
    if let Ok(path) = path.strip_prefix(home) {
        let path = path.to_string_lossy();
        format!("$HOME/{path}")
    } else {
        path.to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_shell_script_path() {
        assert_eq!(
            path_to_shell_script_path(Path::new("/home/rust/.local/bin"), Path::new("/home/rust")),
            "$HOME/.local/bin"
        );
        assert_eq!(
            path_to_shell_script_path(Path::new("/mnt/my_data/bin/"), Path::new("/home/rust")),
            "/mnt/my_data/bin/"
        );
    }
}
