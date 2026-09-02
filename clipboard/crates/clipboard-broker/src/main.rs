mod agent;
mod broker;
mod io;
mod status;
mod wayland;

use std::path::PathBuf;

use clipboard_core::PROTOCOL_VERSION;

use crate::broker::{BrokerConfig, BrokerError, ClipboardMode};

fn main() {
    if let Err(error) = run() {
        eprintln!("clipboard: {error}");
        std::process::exit(error.exit_code());
    }
}

fn run() -> Result<(), BrokerError> {
    let mut args = std::env::args_os();
    let _program = args.next();
    if matches!(args.next().as_deref(), Some(value) if value == "--probe") {
        println!(
            "arch-sway-wslg-clipboard protocol={} arch=x86_64",
            PROTOCOL_VERSION
        );
        return Ok(());
    }

    // The launcher never starts the broker with ARCH_SWAY_WSLG_CLIPBOARD=off.
    let mode = ClipboardMode::from_env()?;
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| BrokerError::Configuration("XDG_RUNTIME_DIR is not set".into()))?
        .join("arch-sway-wslg/clipboard");
    let executable = std::env::current_exe()?;
    let agent = executable
        .parent()
        .ok_or_else(|| {
            BrokerError::Configuration("clipboard executable has no parent directory".into())
        })?
        .join("arch-sway-wslg-clipboard-agent.exe");

    let config = BrokerConfig {
        mode,
        sync_sensitive: std::env::var_os("ARCH_SWAY_WSLG_SYNC_SENSITIVE").as_deref()
            == Some(std::ffi::OsStr::new("1")),
        runtime_dir,
        agent,
    };
    broker::run(config)
}
