// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use crate::{AuxilliaryFile, Kernel, Schema};

/// An entry corresponds to a single kernel, and may have a supplemental
/// cmdline
#[derive(Debug)]
pub struct Entry<'a> {
    pub(crate) kernel: &'a Kernel,

    // Additional cmdline
    cmdline: Option<String>,
}

impl<'a> Entry<'a> {
    /// New entry for the given kernel
    pub fn new(kernel: &'a Kernel) -> Self {
        Self { kernel, cmdline: None }
    }

    /// With the following cmdline
    pub fn with_cmdline(self, cmdline: impl AsRef<str>) -> Self {
        Self {
            cmdline: Some(cmdline.as_ref().to_string()),
            ..self
        }
    }

    /// Return an entry ID, suitable for `.conf` generation
    pub fn id(&self, schema: &Schema) -> String {
        // TODO: For BLS schema, grab something even uniquer (TM)
        let id = match schema {
            Schema::Legacy { os_release, .. } => os_release.name.clone(),
            Schema::Blsforme { os_release } => os_release.id.clone(),
        };
        format!("{id}-{}", &self.kernel.version)
    }

    /// Generate an installed name for the kernel, used by bootloaders
    /// Right now this only returns CBM style IDs
    pub fn installed_kernel_name(&self, schema: &Schema) -> Option<String> {
        match &schema {
            Schema::Legacy { .. } => self
                .kernel
                .image
                .file_name()
                .map(|f| f.to_string_lossy())
                .map(|filename| format!("kernel-{}", filename)),
            Schema::Blsforme { .. } => Some(format!("{}/vmlinuz", self.kernel.version)),
        }
    }

    /// Generate installed asset (aux) name, used by bootloaders
    /// Right now this only returns CBM style IDs
    pub fn installed_asset_name(&self, schema: &Schema, asset: &AuxilliaryFile) -> Option<String> {
        match &schema {
            Schema::Legacy { .. } => match asset.kind {
                crate::AuxilliaryKind::InitRD => asset
                    .path
                    .file_name()
                    .map(|f| f.to_string_lossy())
                    .map(|filename| format!("initrd-{}", filename)),
                _ => None,
            },
            Schema::Blsforme { .. } => {
                let filename = asset.path.file_name().map(|f| f.to_string_lossy())?;
                match asset.kind {
                    crate::AuxilliaryKind::InitRD => Some(format!("{}/{}", &self.kernel.version, filename)),
                    _ => None,
                }
            }
        }
    }
}
