// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Bootloader APIs

use std::path::PathBuf;

use thiserror::Error;

use crate::{manager::Mounts, Configuration, Entry, Firmware, Schema};

pub mod systemd_boot;

/// Bootloader errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("missing bootloader file: {0}")]
    MissingFile(&'static str),

    #[error("missing mountpoint: {0}")]
    MissingMount(&'static str),

    #[error("error: {0}")]
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug)]
pub enum Bootloader<'a, 'b> {
    /// We really only support systemd-boot right now
    Systemd(Box<systemd_boot::Loader<'a, 'b>>),
}

impl<'a, 'b> Bootloader<'a, 'b> {
    /// Construct the firmware-appropriate bootloader manager
    pub(crate) fn new(
        config: &'a Configuration,
        assets: &'b [PathBuf],
        mounts: &'a Mounts,
        firmware: &Firmware,
    ) -> Self {
        match firmware {
            Firmware::UEFI => Bootloader::Systemd(Box::new(systemd_boot::Loader::new(config, assets, mounts))),
            Firmware::BIOS => unimplemented!(),
        }
    }

    /// Sync bootloader to BOOT dir
    pub fn sync(&self) -> Result<(), Error> {
        match &self {
            Bootloader::Systemd(s) => s.sync(),
        }
    }

    /// Install a single kernel, create records for it.
    pub fn install(&self, schema: &Schema, entry: &Entry) -> Result<(), Error> {
        match &self {
            Bootloader::Systemd(s) => s.install(schema, entry),
        }
    }
}
