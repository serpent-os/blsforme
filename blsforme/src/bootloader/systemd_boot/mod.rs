// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! systemd-boot management and interfaces

use std::path::PathBuf;

use crate::{
    file_utils::{changed_files, copy_atomic_vfat, PathExt},
    manager::Mounts,
    Configuration, Entry, Schema,
};

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
            (
                x64_efi.clone(),
                esp.join_insensitive("EFI")
                    .join_insensitive("Boot")
                    .join_insensitive("BOOTX64.EFI"),
            ),
            (
                x64_efi.clone(),
                esp.join_insensitive("EFI")
                    .join_insensitive("systemd")
                    .join_insensitive("systemd-bootx64.efi"),
            ),
        ];

        for (source, dest) in changed_files(targets.as_slice()) {
            copy_atomic_vfat(source, dest)?;
        }

        Ok(())
    }

    /// Install a kernel to the ESP or XBOOTLDR, write a config for it
    pub(super) fn install(&self, schema: &Schema, entry: &Entry) -> Result<(), super::Error> {
        let base = if let Some(xbootldr) = self.mounts.xbootldr.as_ref() {
            xbootldr.clone()
        } else if let Some(esp) = self.mounts.esp.as_ref() {
            esp.clone()
        } else {
            return Err(super::Error::MissingMount("ESP (/efi)"));
        };
        let loader_id = base
            .join_insensitive("loader")
            .join_insensitive("entries")
            .join_insensitive(entry.id(schema))
            .with_extension(".conf");
        log::trace!("writing entry: {}", loader_id.display());
        unimplemented!()
    }
}
