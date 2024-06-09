// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Kernel abstraction

use std::path::PathBuf;
#[derive(Debug)]
pub struct Kernel {
    pub version: String,

    /// vmlinuz path
    pub image: PathBuf,

    /// All of the initrds
    pub initrd: Vec<AuxilliaryFile>,

    /// Any non-initrd, auxillary files
    pub extras: Vec<AuxilliaryFile>,
}

/// Denotes the kind of auxillary file
#[derive(Debug)]
pub enum AuxilliaryKind {
    Cmdline,
    InitRD,
    SystemMap,
    Config,
}

/// Any non-image file
#[derive(Debug)]
pub struct AuxilliaryFile {
    pub path: PathBuf,
    pub kind: AuxilliaryKind,
}
