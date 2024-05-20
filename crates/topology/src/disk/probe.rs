// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Disk probe/query APIs

use std::path::PathBuf;

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
}

#[cfg(test)]
mod tests {
    #[test]
    fn constructor() {
        let p = crate::disk::builder::new().build().expect("What");
        eprintln!("p = {:?}", p.mounts.iter().collect::<Vec<_>>());
    }
}
