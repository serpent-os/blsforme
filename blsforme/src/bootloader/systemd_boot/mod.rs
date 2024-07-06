// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! systemd-boot management and interfaces

use std::path::PathBuf;

use crate::{manager::Mounts, Configuration};

pub mod interface;

/// systemd specific bootloader behaviours
/// NOTE: Currently secure boot is NOT supported (or fbx64)
#[derive(Debug)]
pub struct Loader<'a, 'b> {
    /// system configuration
    config: &'a Configuration,
    assets: &'b [PathBuf],
    mounts: &'a Mounts,
}

impl<'a, 'b> Loader<'a, 'b> {
    /// Construct a new systemd boot loader manager
    pub(super) fn new(config: &'a Configuration, assets: &'b [PathBuf], mounts: &'a Mounts) -> Self {
        Self { config, assets, mounts }
    }

    /// Sync bootloader to ESP (not XBOOTLDR..)
    pub(super) fn sync(&self) -> Result<(), super::Error> {
        eprintln!("Off i sync");
        let x64_efi = self
            .assets
            .iter()
            .find(|p| p.ends_with("systemd-bootx64.efi"))
            .ok_or(super::Error::MissingFile("systemd-bootx64.efi"))?;
        log::debug!("discovered main efi asset: {}", x64_efi.display());

        let esp = self
            .mounts
            .esp
            .as_ref()
            .ok_or(super::Error::MissingMount("ESP (/efi)"))?;
        // Copy systemd-bootx64.efi into these locations
        let targets = vec![
            esp.join("EFI").join("Boot").join("BOOTX64.efi"),
            esp.join("EFI").join("systemd").join("systemd-bootx64.efi"),
        ];

        log::info!("TODO: Copy from {x64_efi:?} to {targets:?}");
        unimplemented!()
    }
}
