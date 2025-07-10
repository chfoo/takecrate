//! Example on how to integrate Takecreate into your CLI application
use std::fs::File;

use clap::{Parser, Subcommand};
use regex_lite::Regex;
use takecrate::{
    inst::{InstallConfig, PackageManifest},
    manifest::{AppId, FileType},
};
use tracing::level_filters::LevelFilter;

/// Entry point
pub fn main() -> anyhow::Result<()> {
    init_logging()?;

    // To complement our debug logging, we also use a wrapped main to log
    // any errors from it
    let result = main2();

    if let Err(error) = &result {
        tracing::error!(?error, "main error");
    }

    result
}

fn main2() -> anyhow::Result<()> {
    let manifest = create_app_package_manifest()?;

    // First, check if we should behave as an automatic guided installer:
    if is_installer() {
        takecrate::install_interactive(&manifest)?;

        return Ok(());
    }

    // Otherwise, we continue normally:
    let args = Args::parse();

    match &args.command {
        Command::Hello => {
            println!("Hello world!");

            let manifest = takecrate::manifest(&manifest.app_id)?;
            let content = std::fs::read_to_string(manifest.app_paths.data.join("test.txt"))?;
            println!("test.txt: {content}");
        }
        Command::Self_(self_args) => match self_args.command {
            SelfCommand::Install { quiet } => {
                if quiet {
                    // Automatically install to user account by default
                    let config = InstallConfig::new_user()?;
                    takecrate::install(&manifest, &config)?;
                } else {
                    // Otherwise, show guided install
                    takecrate::install_interactive(&manifest)?;
                }
            }
            SelfCommand::Uninstall { quiet } => {
                if quiet {
                    // Automatically uninstall
                    takecrate::uninstall(&manifest.app_id)?;
                } else {
                    // Otherwise, prompt the user to confirm
                    takecrate::uninstall_interactive(&manifest.app_id)?;
                }
            }
        },
    }

    Ok(())
}

/// Returns whether the executable's file name is suffixed with "installer" and
/// no command line arguments have been provided by the user.
fn is_installer() -> bool {
    // We ignore the first argument because it is usually the executable path
    // or some other string.
    // We check if any user supplied arguments are provided after the first
    // argument.
    if std::env::args_os().len() > 1 {
        return false;
    }

    // Remove file extension, if any, and check if it ends with the
    // installer prefix (separated by dot, space, underscore, or hyphen).
    let name = std::env::current_exe().unwrap_or_default();
    let name = if !std::env::consts::EXE_SUFFIX.is_empty() {
        name.file_stem().unwrap_or(name.as_os_str())
    } else {
        name.as_os_str()
    };
    let name = name.to_string_lossy();

    let pattern = Regex::new(r"(?i:[. _-]installer)$").unwrap();
    pattern.is_match(&name)
}

/// Initialize logging for debugging
fn init_logging() -> anyhow::Result<()> {
    let log_filename = format!("takecrate_example_installer_{}.log", whoami::username());
    let log_file = File::options()
        .create(true)
        .append(true)
        .open(tempfile::env::temp_dir().join(log_filename))?;

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .with_writer(log_file)
        .init();

    Ok(())
}

/// Create the necessary installer config structs
fn create_app_package_manifest() -> anyhow::Result<PackageManifest> {
    // A unique package name
    let app_id = AppId::new("example.takecrate.takecrate-example-installer")?;

    // Description of the main binary with the "installer" suffix removed
    // and command line arguments on how to uninstall
    let mut manifest = PackageManifest::new(&app_id)
        .with_self_exe_renamed("takecrate-example".to_string() + std::env::consts::EXE_SUFFIX)?
        .with_interactive_uninstall_args(&["self", "uninstall"])
        .with_quiet_uninstall_args(&["self", "uninstall", "--quiet"]);

    // Information for the installer/uninstaller and OS app entries (if applicable)
    manifest.app_metadata.display_name = "Takecrate Example Installer".to_string();
    manifest.app_metadata.display_version = "1.0.0".to_string();

    // Demonstration of including additional files.
    // The source file path is relative to the binary's directory.
    let manifest = manifest.with_file_entry("test.txt", FileType::Data)?;

    Ok(manifest)
}

// Clap arguments:
#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Says hello
    Hello,
    /// Self installer commands
    Self_(SelfArgs),
}

#[derive(Parser, Debug)]
struct SelfArgs {
    #[command(subcommand)]
    command: SelfCommand,
}

#[derive(Debug, Subcommand)]
enum SelfCommand {
    /// Installer
    Install {
        /// Install without prompting the user
        #[arg(long)]
        quiet: bool,
    },
    /// Uninstaller
    Uninstall {
        /// Uninstall without prompting the user
        #[arg(long)]
        quiet: bool,
    },
}
