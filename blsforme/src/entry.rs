// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use crate::Kernel;

/// An entry corresponds to a single kernel, and may have a supplemental
/// cmdline
#[derive(Debug)]
pub struct Entry<'a> {
    kernel: &'a Kernel,

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
    pub fn id(&self) -> String {
        // TODO: For CBM schema, grab `.NAME` from os-release
        //       For BLS schema, grab something even uniquer (TM)
        if let Some(variant) = self.kernel.variant.as_ref() {
            format!("unknown-{variant}-{}", &self.kernel.version)
        } else {
            format!("unknown-{}", &self.kernel.version)
        }
    }
}
