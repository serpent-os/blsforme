// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Disk probe/query APIs

use std::{
    fs,
    path::{Path, PathBuf},
};

use nix::sys::stat;

use super::mounts::Table;

/// A Disk probe to query disks
#[derive(Debug)]
pub struct Probe {
    /// Root of all operations
    pub(super) root: PathBuf,

    /// location of /sys
    pub(super) sysfs: PathBuf,

    /// location of /dev
    pub(super) devfs: PathBuf,

    /// location of /proc
    pub(super) procfs: PathBuf,

    /// Mountpoints
    pub(super) mounts: Table,
}

impl Probe {
    /// Initial startup loads
    /// TODO: If requested, pvscan/vgscan/lvscan
    pub(super) fn init_scan(&mut self) -> Result<(), super::Error> {
        let mounts = Table::new_from_path(self.procfs.join("self").join("mounts"))?;
        self.mounts = mounts;

        Ok(())
    }

    /// Resolve a device by mountpoint
    pub fn get_device_from_mountpoint(
        &self,
        mountpoint: impl AsRef<Path>,
    ) -> Result<String, super::Error> {
        let mountpoint = fs::canonicalize(mountpoint.as_ref())?;

        // Attempt to stat the device
        let stat = stat::lstat(&mountpoint)?;
        let device_path = self.devfs.join("block").join(format!(
            "{}:{}",
            stat::major(stat.st_dev),
            stat::minor(stat.st_dev)
        ));

        // Return by stat path if possible, otherwise fallback to mountpoint device
        if device_path.exists() {
            Ok(fs::canonicalize(&device_path)?
                .to_string_lossy()
                .to_string())
        } else {
            // Find matching mountpoint
            let matching_device = self
                .mounts
                .iter()
                .find(|m| PathBuf::from(m.mountpoint) == mountpoint)
                .ok_or_else(|| super::Error::UnknownMount(mountpoint))?;
            // TODO: Handle `ZFS=`, and composite bcachefs mounts (dev:dev1:dev2)
            Ok(matching_device.device.to_owned())
        }
    }

    /// Retrieve the parent device, such as the disk of a partition, if possible
    pub fn get_device_parent(&self, device: impl AsRef<Path>) -> Option<PathBuf> {
        let device = fs::canonicalize(device.as_ref()).ok()?;
        let child = fs::canonicalize(
            device
                .file_name()
                .map(|f| self.sysfs.join("class").join("block").join(f))?,
        )
        .ok()?;
        let parent = child.parent()?.file_name()?;
        fs::canonicalize(self.devfs.join(parent)).ok()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn constructor() {
        let p = crate::disk::builder::new().build().expect("What");
        eprintln!("p = {:?}", p.mounts.iter().collect::<Vec<_>>());
        let device = p.get_device_from_mountpoint("/").expect("need /");
        eprintln!("root = {}", &device);
        eprintln!("parent = {:?}", p.get_device_parent(&device));
    }
}
