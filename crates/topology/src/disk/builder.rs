// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Builder API for constructing the Probe
use std::fs;

use crate::disk::probe::Probe;

use super::mounts::Table;

/// Builder pattern for a Probe
pub struct Builder<'a> {
    sysfs: &'a str,
    devfs: &'a str,
    procfs: &'a str,
}

/// Generate default builder
pub fn new<'a>() -> Builder<'a> {
    Builder {
        sysfs: "/sys",
        devfs: "/dev",
        procfs: "/proc",
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        self::new()
    }
}

impl<'a> Builder<'a> {
    // sysfs directory
    pub fn with_sysfs(self, sysfs: &'a str) -> Self {
        Self { sysfs, ..self }
    }

    /// devfs directory
    pub fn with_devfs(self, devfs: &'a str) -> Self {
        Self { devfs, ..self }
    }

    // procfs directory
    pub fn with_procfs(self, procfs: &'a str) -> Self {
        Self { procfs, ..self }
    }

    /// Return a newly built Probe
    /// Note: All input paths will be verified
    pub fn build(self) -> Result<Probe, super::Error> {
        let mut result = Probe {
            sysfs: fs::canonicalize(self.sysfs)?,
            devfs: fs::canonicalize(self.devfs)?,
            procfs: fs::canonicalize(self.procfs)?,
            mounts: Table::default(),
        };
        result.init_scan()?;
        Ok(result)
    }
}
