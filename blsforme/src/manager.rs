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

use crate::{BootEnvironment, Configuration, Error, Kernel, Root};

#[derive(Debug)]
struct Mounts {
    xbootldr: Option<PathBuf>,
    esp: Option<PathBuf>,
}

/// Encapsulate the entirety of the boot management core APIs
#[derive(Debug)]
pub struct Manager<'a> {
    config: &'a Configuration,

    /// OS provided kernels
    system_kernels: Vec<Kernel>,

    /// Our detected boot environment
    boot_env: BootEnvironment,

    mounts: Mounts,
}

impl<'a> Manager<'a> {
    /// Construct a new blsforme::Manager with the given configuration
    pub fn new(config: &'a Configuration) -> Result<Self, Error> {
        // Probe the rootfs device managements
        let probe = disk::Builder::default().build()?;
        let root = probe.get_rootfs_device(config.root.path())?;
        log::info!("root = {:?}", root.cmd_line());

        // Grab parent disk, establish disk environment setup
        let disk_parent = probe.get_device_parent(root.path);
        let boot_env = BootEnvironment::new(&probe, disk_parent, config)?;
        log::trace!("boot env: {boot_env:?}");

        let mut mounts = Mounts {
            xbootldr: if let Some(point) = boot_env.xboot_mountpoint.as_ref() {
                Some(point.clone())
            } else {
                Some(config.root.path().join("boot"))
            },
            esp: if let Some(point) = boot_env.esp_mountpoint.as_ref() {
                Some(point.clone())
            } else {
                Some(config.root.path().join("efi"))
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
            system_kernels: vec![],
            boot_env,
            mounts,
        })
    }

    /// Set the system kernels to use for sync operations
    pub fn with_kernels(self, kernels: Vec<Kernel>) -> Self {
        Self {
            system_kernels: kernels,
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
