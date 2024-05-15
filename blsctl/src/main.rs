// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Provides a CLI compatible with `clr-boot-manager` to be used as a drop-in
//! replacement for Solus.

use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use blsforme::{os_release::OsRelease, topology::Topology, Configuration, Root};
use clap::{Parser, Subcommand};
use color_eyre::{
    eyre::{eyre, Context},
    Section,
};
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

    /// Status information (debugging)
    Status,
}

/// Determine the schema to utilise when scanning for kernels
#[derive(Debug)]
enum RootSchema {
    /// clr-boot-manager era, fixed namespace
    CBM(&'static str),

    /// blsforme schema
    BLS4,
}

/// Query the schema we need to use for pre BLS schema installations
fn query_schema(config: &Configuration) -> color_eyre::Result<RootSchema> {
    let query_paths = vec![
        config.root.path().join("run").join("os-release"),
        config.root.path().join("etc").join("os-release"),
        config
            .root
            .path()
            .join("usr")
            .join("lib")
            .join("os-release"),
    ];

    for p in query_paths {
        if p.exists() {
            log::trace!("Reading os-release from: {}", p.display());
            let text = fs::read_to_string(p)?;
            let os_rel = OsRelease::from_str(&text)?;

            match os_rel.id.as_str() {
                "solus" => {
                    if os_rel.version.name.is_some_and(|v| v.starts_with('4')) {
                        log::trace!("Legacy schema due to Solus 4 installation");
                        return Ok(RootSchema::CBM("com.solus-project"));
                    } else {
                        return Ok(RootSchema::BLS4);
                    }
                }
                "clear-linux-os" => {
                    log::trace!("Legacy schema due to Clear Linux OS installation");
                    return Ok(RootSchema::CBM("org.clearlinux"));
                }
                _ => return Ok(RootSchema::BLS4),
            }
        }
    }

    Err(
        eyre!("Unable to detect the Linux distribution").with_warning(|| "A valid os-release file is required to detect the kernel schema. It is not possible to proceed without it")
    )
}

fn inspect_root(config: &Configuration) -> color_eyre::Result<Topology> {
    let probe = Topology::probe(config)
        .wrap_err(format!(
            "Unable to probe topology and block device for `{}`",
            config.root.path().display()
        ))
        .with_note(|| "Please make sure that the path definitely exists and is readable")?;
    log::trace!("Topology result: {probe:?}");

    log::info!("Using rootfs device: {}", probe.rootfs.path.display());
    log::info!("Additional /proc/cmdline: {}", probe.rootfs.root_cmdline());

    let schema = query_schema(config)?;
    log::info!("Root Schema: {schema:?}");

    Ok(probe)
}

fn main() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .issue_url("https://github.com/serpent-os/blsforme/issues/new")
        .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
        .issue_filter(|_| true)
        .install()?;

    formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

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
    log::info!("Inspecting root device: {}", config.root.path().display());

    match res.command {
        Commands::Version => todo!(),
        Commands::ReportBooted => todo!(),
        Commands::RemoveKernel => todo!(),
        Commands::MountBoot => todo!(),
        Commands::Update => todo!(),
        Commands::SetTimeout { timeout: _ } => todo!(),
        Commands::GetTimeout => todo!(),
        Commands::SetKernel { kernel: _ } => todo!(),
        Commands::ListKernels => todo!(),
        Commands::Status => {
            let _ = inspect_root(&config)?;
        }
    }

    Ok(())
}
