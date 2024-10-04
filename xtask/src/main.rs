use core::str;
use std::{io::Write, path::PathBuf};

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Debug, Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    RunExampleInstaller {
        #[arg(long, short, default_value = "")]
        cargo_args: String,
        #[arg(long, short, default_value = "")]
        program_args: String,
    },
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::RunExampleInstaller {
            cargo_args,
            program_args,
        } => run_example_installer(cargo_args, program_args),
    }
}

fn run_example_installer(cargo_args: String, program_args: String) -> anyhow::Result<()> {
    let cargo = std::env::var("CARGO")?;
    let project_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?).join("..");

    let mut args = vec![
        "build",
        "--message-format=json",
        "--example",
        "takecrate_example_installer",
    ];
    args.extend(cargo_args.split_whitespace());
    let output = std::process::Command::new(&cargo).args(args).output()?;
    let stdout = str::from_utf8(&output.stdout)?;

    if !output.status.success() {
        std::io::stdout().write_all(&output.stdout)?;
        std::io::stderr().write_all(&output.stderr)?;
        anyhow::bail!("build info failed");
    }

    let mut executable_path = PathBuf::new();

    for line in stdout.lines() {
        let value = serde_json::from_str::<Value>(line)?;
        let reason = value
            .as_object()
            .and_then(|obj| obj.get("reason").and_then(|val| val.as_str()));
        let executable = value
            .as_object()
            .and_then(|obj| obj.get("executable").and_then(|val| val.as_str()));

        if reason == Some("compiler-artifact") {
            if let Some(executable) = executable {
                executable_path = PathBuf::from(executable);
            }
        }
    }

    anyhow::ensure!(executable_path.is_file());

    dbg!(&project_dir, &executable_path);

    std::fs::copy(
        project_dir.join("examples/test.txt"),
        executable_path.parent().unwrap().join("test.txt"),
    )?;

    let mut args = vec!["run", "--example", "takecrate_example_installer"];
    args.extend(cargo_args.split_whitespace());
    args.push("--");
    args.extend(program_args.split_whitespace());

    std::process::Command::new(&cargo).args(args).status()?;

    Ok(())
}
