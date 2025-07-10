//! Test installing in a temporary directory.
use takecrate::{
    error::InstallerErrorKind,
    inst::{InstallConfig, PackageManifest},
    manifest::AppId,
    os::AccessScope,
    path::AppPathPrefix,
};
use tempfile::NamedTempFile;

fn make_data_file() -> NamedTempFile {
    let dir = takecrate::os::current_exe_dir().unwrap();
    tempfile::Builder::new()
        .prefix("data-")
        .suffix(".tmp")
        .tempfile_in(dir)
        .unwrap()
}

#[test_log::test]
fn test_single_dir() {
    let dest_dir = tempfile::tempdir().unwrap();
    let data_file = make_data_file();

    let app_id =
        AppId::new("takecrate.tests.takecrate_tests_directory_install_single_dir").unwrap();
    let package_manifest = PackageManifest::new(&app_id)
        .with_self_exe()
        .unwrap()
        .with_file_entry(
            data_file.path().file_name().unwrap(),
            takecrate::manifest::FileType::Data,
        )
        .unwrap();

    let mut config = InstallConfig::default();
    config.access_scope = AccessScope::User;
    config.source_dir = takecrate::os::current_exe_dir().unwrap();
    config.destination = AppPathPrefix::SingleDir(dest_dir.path().to_path_buf());
    config.modify_os_search_path = false;

    takecrate::install(&package_manifest, &config).unwrap();

    let disk_manifest = takecrate::manifest(&app_id).unwrap();

    let bin_file_path = &disk_manifest.main_executable().unwrap().path;
    let data_file_path = &disk_manifest
        .app_paths
        .data
        .join(data_file.path().file_name().unwrap());

    assert!(disk_manifest.app_paths.executable.is_dir());
    assert!(disk_manifest.app_paths.data.is_dir());

    assert!(bin_file_path.is_file());
    assert_eq!(
        bin_file_path,
        &dest_dir
            .path()
            .join("bin")
            .join(takecrate::os::current_exe_name().unwrap())
    );

    assert!(data_file_path.is_file());
    assert_eq!(
        data_file_path,
        &dest_dir.path().join(data_file.path().file_name().unwrap())
    );

    takecrate::uninstall(&app_id).unwrap();

    let result = takecrate::uninstall(&app_id);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        InstallerErrorKind::NotInstalled
    ));

    dest_dir.close().unwrap();
}

#[test_log::test]
fn test_custom_unix() {
    let dest_dir = tempfile::tempdir().unwrap();
    let data_file = make_data_file();

    let app_id =
        AppId::new("takecrate.tests.takecrate_tests_directory_install_custom_unix").unwrap();
    let package_manifest = PackageManifest::new(&app_id)
        .with_self_exe()
        .unwrap()
        .with_file_entry(
            data_file.path().file_name().unwrap(),
            takecrate::manifest::FileType::Data,
        )
        .unwrap();

    let mut config = InstallConfig::default();
    config.access_scope = AccessScope::User;
    config.source_dir = takecrate::os::current_exe_dir().unwrap();
    config.destination = AppPathPrefix::CustomUnix(dest_dir.path().to_path_buf());
    config.modify_os_search_path = false;

    takecrate::install(&package_manifest, &config).unwrap();

    let disk_manifest = takecrate::manifest(&app_id).unwrap();

    let bin_file_path = &disk_manifest.main_executable().unwrap().path;
    let data_file_path = &disk_manifest
        .app_paths
        .data
        .join(data_file.path().file_name().unwrap());

    assert!(disk_manifest.app_paths.executable.is_dir());
    assert!(disk_manifest.app_paths.data.is_dir());

    assert!(bin_file_path.is_file());
    assert_eq!(
        bin_file_path,
        &dest_dir
            .path()
            .join("bin")
            .join(takecrate::os::current_exe_name().unwrap())
    );

    assert!(data_file_path.is_file());
    assert_eq!(
        data_file_path,
        &dest_dir
            .path()
            .join("share/takecrate_tests_directory_install_custom_unix")
            .join(data_file.path().file_name().unwrap())
    );

    takecrate::uninstall(&app_id).unwrap();

    let result = takecrate::uninstall(&app_id);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err().kind(),
        InstallerErrorKind::NotInstalled
    ));

    dest_dir.close().unwrap();
}
