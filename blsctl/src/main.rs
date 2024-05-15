// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Provides a CLI compatible with `clr-boot-manager` to be used as a drop-in
//! replacement for Solus.

use std::path::{Path, PathBuf};

use blsforme::{topology::Topology, Configuration, Root};
use clap::{Parser, Subcommand};
use color_eyre::{eyre::Context, Section};
use pretty_env_logger::formatted_builder;

/// Boot Loader Specification compatible kernel/initrd/cmdline management
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Override base path for all boot management operations
    #[arg(short, long, global = true)]
    path: Option<PathBuf>,

    /// Force running in image mode (scripting integration)
    #[arg(short, long, global = true)]
    image: bool,

    /// Do not allow updating EFI vars
    #[arg(short, long, global = true)]
    no_efi_update: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Print version and exit
    Version,

    /// Report currently running kernel as successfully booting
    ReportBooted,

    /// Remove specified kernel from the system
    RemoveKernel,

    /// Mount the `$BOOT` directories
    MountBoot,

    /// Configure the `$BOOT` directories for next boot
    Update,

    /// Set the bootloader timeout value
    SetTimeout { timeout: u64 },

    /// Retrieve the bootloader timeout value
    GetTimeout,

    /// Set the kernel that will be used at next boot
    SetKernel { kernel: String },

    /// List kernels on `$BOOT`
    ListKernels,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .issue_url("https://github.com/serpent-os/blsforme/issues/new")
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .issue_filter(|kind| match kind {
            color_eyre::ErrorKind::NonRecoverable(_) => false,
            color_eyre::ErrorKind::Recoverable(_) => true,
        })
        .install()?;

    formatted_builder().init();

    let res = Cli::parse();
    let root = if res.image {
        // forced image mode
        Root::Image(res.path.unwrap_or("/".into()))
    } else if let Some(path) = res.path {
        // Path provided, native only if it is `/`
        if path.as_path() == Path::new("/") {
            Root::Native(path)
        } else {
            Root::Image(path)
        }
    } else {
        // Native operation
        Root::Native("/".into())
    };

    let config = Configuration { root };

    log::trace!("Using configuration: {config:?}");
    println!(
        " 🔎 Inspecting root device: {}",
        config.root.path().display()
    );

    match res.command {
        Commands::Version => todo!(),
        Commands::ReportBooted => todo!(),
        Commands::RemoveKernel => todo!(),
        Commands::MountBoot => todo!(),
        Commands::Update => {
            let probe = Topology::probe(&config)
                .wrap_err(format!(
                    "Unable to probe topology and block device for `{}`",
                    config.root.path().display()
                ))
                .with_note(|| "Please make sure that the path definitely exists and is readable")?;
            log::info!("Topology result: {probe:?}");

            println!();
            println!(
                "    *  Found rootfs device: {}",
                probe.rootfs.path.display()
            );
            println!(
                "    *  Additional `/proc/cmdline`: {}",
                probe.rootfs.root_cmdline()
            );
        }
        Commands::SetTimeout { timeout: _ } => todo!(),
        Commands::GetTimeout => todo!(),
        Commands::SetKernel { kernel: _ } => todo!(),
        Commands::ListKernels => todo!(),
    }

    Ok(())
}
