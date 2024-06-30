// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Kernel abstraction

use std::path::PathBuf;

/// A kernel is the primary bootable element that we care about, ie
/// the vmlinuz file. It also comes with a set of auxilliary files
/// that are required for a fully working system, but specifically
/// dependent on that kernel version.
#[derive(Debug)]
pub struct Kernel {
    /// Matches the `uname -r` of the kernel, should be uniquely encoded by release/variant
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
    /// A cmdline snippet
    Cmdline,

    /// An initial ramdisk
    InitRD,

    /// System.map file
    SystemMap,

    /// .config file
    Config,
}

/// An additional file required to be shipped with the kernel,
/// such as initrds, system maps, etc.
#[derive(Debug)]
pub struct AuxilliaryFile {
    pub path: PathBuf,
    pub kind: AuxilliaryKind,
}
