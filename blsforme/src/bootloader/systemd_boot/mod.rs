// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! systemd-boot management and interfaces

use std::{
    fs::{self, create_dir_all},
    path::PathBuf,
};

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
    #[allow(dead_code)]
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
    pub(super) fn install(&self, cmdline: &str, schema: &Schema, entry: &Entry) -> Result<(), super::Error> {
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
            .with_extension("conf");
        log::trace!("writing entry: {}", loader_id.display());

        // Old schema used `com.*`, now we use `$id` from os-release
        let asset_dir_base = match schema {
            Schema::Legacy { namespace, .. } => namespace.to_string(),
            Schema::Blsforme { os_release } => os_release.id.clone(),
        };

        let asset_dir = base.join_insensitive("EFI").join_insensitive(&asset_dir_base);

        // vmlinuz primary path
        let vmlinuz = asset_dir.join_insensitive(
            entry
                .installed_kernel_name(schema)
                .ok_or_else(|| super::Error::MissingFile("vmlinuz"))?,
        );
        // initrds requiring install
        let initrds = entry
            .kernel
            .initrd
            .iter()
            .filter_map(|asset| {
                Some((
                    asset.path.clone(),
                    asset_dir.join_insensitive(entry.installed_asset_name(schema, asset)?),
                ))
            })
            .collect::<Vec<_>>();
        log::trace!("with kernel path: {}", vmlinuz.display());
        log::trace!("with initrds: {:?}", initrds);

        // build up the total changeset
        let mut changeset = vec![(entry.kernel.image.clone(), vmlinuz)];
        changeset.extend(initrds);

        // Determine which need copying now.
        let needs_writing = changed_files(changeset.as_slice());
        log::trace!("requires update: {needs_writing:?}");

        // Donate them to disk
        for (source, dest) in needs_writing {
            copy_atomic_vfat(source, dest)?;
        }

        let loader_config = self.generate_entry(&asset_dir_base, cmdline, schema, entry);
        log::trace!("loader config: {loader_config}");

        let entry_dir = base.join_insensitive("loader").join_insensitive("entries");
        if !entry_dir.exists() {
            create_dir_all(entry_dir)?;
        }

        // TODO: Hash compare and dont obliterate!
        fs::write(loader_id, loader_config)?;

        Ok(())
    }

    /// Generate a usable loader config entry
    fn generate_entry(&self, asset_dir: &str, cmdline: &str, schema: &Schema, entry: &Entry) -> String {
        let initrd = if entry.kernel.initrd.is_empty() {
            "\n".to_string()
        } else {
            let initrds = entry
                .kernel
                .initrd
                .iter()
                .filter_map(|asset| {
                    Some(format!(
                        "\ninitrd /EFI/{asset_dir}/{}",
                        entry.installed_asset_name(schema, asset)?
                    ))
                })
                .collect::<String>();
            format!("\n{}", initrds)
        };
        let title = if let Some(pretty) = schema.os_release().meta.pretty_name.as_ref() {
            format!("{pretty} ({})", entry.kernel.version)
        } else {
            format!("{} ({})", schema.os_release().name, entry.kernel.version)
        };
        let vmlinuz = entry.installed_kernel_name(schema).expect("linux go boom");
        let options = "".to_owned() + cmdline;
        format!(
            r###"title {title}
linux /EFI/{asset_dir}/{}{}
options {}
"###,
            vmlinuz, initrd, options
        )
    }
}
