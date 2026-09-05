use std::{
    path::{Component, PathBuf},
    process::Command,
};

use clap::Parser;

use crate::{
    cli::{CommandExt as _, derive_adb_path},
    shell::{Shell, Verbosity},
};

#[derive(Debug, Parser, Clone)]
struct RunnerArgs {
    #[arg(long, env = "CARGO_NDK_ADB_SERIAL")]
    /// "Serial number" of the device to use for testing (e.g. "emulator-5554" or "0123456789ABCDEF")
    ///
    /// You can find the serial number of your device by running `adb devices`.
    ///
    /// If not set, the first available device will be used.
    adb_serial: Option<String>,

    #[arg(short)]
    /// Enable verbose output
    verbose: bool,

    #[arg(short)]
    /// Enable quiet output (no output except errors)
    quiet: bool,

    /// Path to the binary to run on the device
    executable: PathBuf,

    #[arg(allow_hyphen_values = true)]
    /// Arguments to be run
    runner_args: Vec<String>,
}

fn prepend_path(prefix: &PathBuf, path: &PathBuf) -> PathBuf {
    let mut result = prefix.clone();
    for component in path.components() {
        match component {
            Component::RootDir | Component::Prefix(_) => continue,
            _ => result.push(component.as_os_str()),
        }
    }
    result
}

pub fn run(args: Vec<String>) -> anyhow::Result<()> {
    let mut shell = Shell::new();

    let args = RunnerArgs::try_parse_from(args).unwrap_or_else(|e| {
        shell.error(e).unwrap();
        std::process::exit(2);
    });

    let adb_serial = args.adb_serial.as_deref();
    let verbosity = if args.verbose {
        Verbosity::Verbose
    } else if args.quiet {
        Verbosity::Quiet
    } else {
        Verbosity::Normal
    };

    shell.set_verbosity(verbosity);

    // Get adb path
    let adb_path = match derive_adb_path(&mut shell) {
        Ok(path) => path,
        Err(e) => {
            shell.error(e)?;
            std::process::exit(1);
        }
    };

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap());

    let device_root = PathBuf::from("/data/local/tmp");
    let device_project_path = prepend_path(&device_root, &manifest_dir);

    let mkdir_status = Command::new(&adb_path)
        .with_serial(adb_serial)
        .arg("shell")
        .arg("mkdir")
        .arg("-p")
        .arg(&device_project_path)
        .output()?;

    if !mkdir_status.status.success() {
        shell.error("Failed to create directory on device")?;
        eprintln!("{}", std::str::from_utf8(&mkdir_status.stderr)?.trim());
        std::process::exit(mkdir_status.status.code().unwrap_or(1));
    }

    // Push binary to device
    let device_bin_path = prepend_path(&device_project_path, &args.executable);

    // Ugly but works
    shell.verbose(|shell| {
        shell.status_header("Pushing")?;
        shell.reset_err()?;
        shell.err().write_fmt(format_args!(
            "binary to device: {}\r",
            device_bin_path.display()
        ))?;
        shell.set_needs_clear(true);
        Ok(())
    })?;

    let push_status = Command::new(&adb_path)
        .with_serial(adb_serial)
        .arg("push")
        .arg(&args.executable)
        .arg(&device_bin_path)
        .output()?;

    if !push_status.status.success() {
        shell.error("Failed to push test binary to device")?;
        eprintln!("{}", std::str::from_utf8(&push_status.stderr)?.trim());
        shell.note("If multiple devices, use --adb-serial to specify one.")?;
        shell.note("Run `adb devices` to see connected devices.")?;
        std::process::exit(push_status.status.code().unwrap_or(1));
    }

    shell.verbose(|shell| {
        shell.status(
            "Pushing",
            format!("binary to device ({})", device_bin_path.display()),
        )
    })?;

    // Make binary executable
    let chmod_status = Command::new(&adb_path)
        .with_serial(adb_serial)
        .arg("shell")
        .arg("chmod")
        .arg("755")
        .arg(&device_bin_path)
        .status()?;

    if !chmod_status.success() {
        shell.error("Failed to make binary executable")?;
        std::process::exit(chmod_status.code().unwrap_or(1));
    }

    shell.reset_err()?;

    let verbosity_arg = match verbosity {
        Verbosity::Quiet => "-q",
        _ => "",
    };

    let run_status = Command::new(&adb_path)
        .with_serial(adb_serial)
        .arg("shell")
        .arg("cd")
        .arg(&device_project_path)
        .arg("&&")
        .arg(&device_bin_path)
        .arg(verbosity_arg)
        .args(&args.runner_args)
        .status()?;

    // Clean up the binary from device
    let _ = Command::new(&adb_path)
        .with_serial(adb_serial)
        .arg("shell")
        .arg("rm")
        .arg(&device_bin_path)
        .status();

    std::process::exit(run_status.code().unwrap_or(1))
}
