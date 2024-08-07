// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Boot loader management entry APIs

use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use nix::mount::{mount, umount, MsFlags};
use topology::disk;

use crate::{bootloader::Bootloader, BootEnvironment, Configuration, Entry, Error, Root, Schema};

#[derive(Debug)]
pub(crate) struct Mounts {
    pub(crate) xbootldr: Option<PathBuf>,
    pub(crate) esp: Option<PathBuf>,
}

/// Encapsulate the entirety of the boot management core APIs
#[derive(Debug)]
pub struct Manager<'a> {
    config: &'a Configuration,

    /// OS provided kernels
    entries: Vec<Entry<'a>>,

    /// Potential bootloader assets, allow impl to filter for right paths
    bootloader_assets: Vec<PathBuf>,

    /// Our detected boot environment
    boot_env: BootEnvironment,

    mounts: Mounts,

    cmdline: String,
}

impl<'a> Manager<'a> {
    /// Construct a new blsforme::Manager with the given configuration
    pub fn new(config: &'a Configuration) -> Result<Self, Error> {
        // Probe the rootfs device managements
        let probe = disk::Builder::default().build()?;
        let root = probe.get_rootfs_device(config.root.path())?;
        // Enforce RW rootfs, will review if other downstreams need something different.
        let cmdline = root.cmd_line() + " rw";
        log::info!("root = {:?}", &cmdline);

        // Grab parent disk, establish disk environment setup
        let disk_parent = probe.get_device_parent(root.path);
        let boot_env = BootEnvironment::new(&probe, disk_parent, config)?;
        log::trace!("boot env: {boot_env:?}");

        let mut mounts = Mounts {
            xbootldr: if let Some(point) = boot_env.xboot_mountpoint.as_ref() {
                Some(point.clone())
            } else if boot_env.xbootldr().is_some() {
                Some(config.root.path().join("boot"))
            } else {
                None
            },
            esp: if let Some(point) = boot_env.esp_mountpoint.as_ref() {
                Some(point.clone())
            } else if boot_env.esp().is_some() {
                Some(config.root.path().join("efi"))
            } else {
                None
            },
        };

        log::trace!("selected mountpoints: {mounts:?}");

        // So, we got a `/boot` mount for ESP, legacy style. We can't stick xbootldr there...
        if let Some(xbootldr) = mounts.xbootldr.as_ref() {
            if let Some(esp) = mounts.esp.as_ref() {
                if esp == xbootldr && boot_env.xbootldr().is_none() {
                    mounts.xbootldr = Some(config.root.path().join("xboot"))
                }
            }
        }

        Ok(Self {
            config,
            entries: vec![],
            bootloader_assets: vec![],
            boot_env,
            mounts,
            cmdline,
        })
    }

    /// Set the system kernels to use for sync operations
    pub fn with_entries(self, entries: impl Iterator<Item = Entry<'a>>) -> Self {
        Self {
            entries: entries.collect::<Vec<_>>(),
            ..self
        }
    }

    /// Update the set of bootloader assets
    pub fn with_bootloader_assets(self, assets: Vec<PathBuf>) -> Self {
        Self {
            bootloader_assets: assets,
            ..self
        }
    }

    /// Mount any required partitions (ESP/XBOOTLDR)
    pub fn mount_partitions(&self) -> Result<Vec<ScopedMount>, Error> {
        let mut mounted_paths = vec![];

        // Stop silly buggers with image based mounting
        if let Root::Image(_) = self.config.root {
            log::warn!("Refusing to auto-mount partitions in image mode");
            return Ok(mounted_paths);
        }

        // Got the ESP, not mounted.
        if let Some(hw) = self.boot_env.esp() {
            if self.boot_env.esp_mountpoint.is_none() {
                let mount_point = self.mounts.esp.clone().ok_or_else(|| Error::NoESP)?;
                mounted_paths.insert(0, self.mount_vfat_partition(hw, &mount_point)?);
            }
        }
        // Got an XBOOTLDR, not mounted..
        if let Some(hw) = self.boot_env.xbootldr() {
            if self.boot_env.xboot_mountpoint.is_none() {
                let mount_point = self.mounts.xbootldr.clone().ok_or_else(|| Error::NoXBOOTLDR)?;
                mounted_paths.insert(0, self.mount_vfat_partition(hw, &mount_point)?);
            }
        }

        Ok(mounted_paths)
    }

    /// Mount an fat filesystem
    #[inline]
    fn mount_vfat_partition(&self, source: &Path, target: &Path) -> Result<ScopedMount, Error> {
        let options: Option<&str> = None;
        if !target.exists() {
            create_dir_all(target)?;
        }
        mount(Some(source), target, Some("vfat"), MsFlags::MS_MGC_VAL, options)?;
        log::info!("Mounted vfat partition {} at {}", source.display(), target.display());
        Ok(ScopedMount {
            point: target.into(),
            mounted: true,
        })
    }

    /// Attempt to sync kernels/bootloader with the targets
    ///
    /// Any already installed kernels will be skipped, and this step
    /// is not responsible for *deleting* any unused kernels
    pub fn sync(&self, schema: &Schema) -> Result<(), Error> {
        if let Root::Image(_) = self.config.root {
            if let Some(esp) = self.boot_env.esp() {
                if self.boot_env.esp_mountpoint.is_none() {
                    return Err(Error::UnmountedESP(esp.clone()));
                }
            }
        }
        // Firstly, get the bootloader updated.
        let bootloader = Bootloader::new(
            self.config,
            &self.bootloader_assets,
            &self.mounts,
            &self.boot_env.firmware,
        );
        bootloader.sync()?;

        // Install every kernel that was passed to us
        for entry in self.entries.iter() {
            bootloader.install(&self.cmdline, schema, entry)?;
        }

        Ok(())
    }
}

/// Encapsulated mountpoint to ensure auto-unmount (Scoped)
pub struct ScopedMount {
    point: PathBuf,
    mounted: bool,
}

impl Drop for ScopedMount {
    fn drop(&mut self) {
        if !self.mounted {
            return;
        }
        self.mounted = true;
        match umount(&self.point) {
            Ok(_) => log::info!("Unmounted {}", self.point.display()),
            Err(err) => log::error!("Failed to umount {}: {}", self.point.display(), err.to_string()),
        }
    }
}
