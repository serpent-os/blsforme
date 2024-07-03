// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Boot loader management entry APIs

use topology::disk;

use crate::{BootEnvironment, Configuration, Error, Kernel};

/// Encapsulate the entirety of the boot management core APIs
pub struct Manager<'a> {
    config: &'a Configuration,

    /// OS provided kernels
    system_kernels: Vec<Kernel>,

    /// Our detected boot environment
    boot_env: BootEnvironment,
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

        Ok(Self {
            config,
            system_kernels: vec![],
            boot_env,
        })
    }

    /// Set the system kernels to use for sync operations
    pub fn with_kernels(self, kernels: Vec<Kernel>) -> Self {
        Self {
            system_kernels: kernels,
            ..self
        }
    }
}
