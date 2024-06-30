// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Kernel abstraction

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Control kernel discovery mechanism
#[derive(Debug)]
pub enum Schema {
    /// Legacy (clr-boot-manager style) schema
    Legacy(&'static str),

    /// Modern schema (actually has a schema.)
    Blsforme,
}

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

    /// Recorded variant type
    pub variant: Option<String>,
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

impl Schema {
    /// Given a set of kernel-like paths, yield all potential kernels within them
    /// This should be a set of `/usr/lib/kernel` paths. Use glob or appropriate to discover.
    pub fn discover_system_kernels(&self, paths: impl Iterator<Item = impl AsRef<Path>>) -> Vec<Kernel> {
        match &self {
            Schema::Legacy(name) => Self::legacy_kernels(name, paths),
            Schema::Blsforme => Self::blsforme_kernels(paths),
        }
    }

    /// Discover any legacy kernels
    fn legacy_kernels(namespace: &'static str, paths: impl Iterator<Item = impl AsRef<Path>>) -> Vec<Kernel> {
        let paths = paths.collect::<Vec<_>>();
        // First up, find kernels. They start with the prefix..
        let candidates = paths.iter().filter_map(|p| {
            if p.as_ref().file_name()?.to_str()?.starts_with(namespace) {
                Some(p)
            } else {
                None
            }
        });

        let mut kernels = BTreeMap::new();

        // TODO: Make use of release
        for cand in candidates {
            let item = cand.as_ref();
            if let Some(file_name) = item.file_name().map(|f| f.to_string_lossy().to_string()) {
                let (left, right) = file_name.split_at(namespace.len() + 1);
                assert!(left.ends_with('.'));
                if let Some((variant, full_version)) = right.split_once('.') {
                    if let Some((_version, _release)) = full_version.rfind('-').map(|i| full_version.split_at(i)) {
                        log::trace!("discovered vmlinuz: {file_name}");
                        kernels.insert(
                            full_version.to_string(),
                            Kernel {
                                version: full_version.to_string(),
                                image: item.into(),
                                initrd: vec![],
                                extras: vec![],
                                variant: Some(variant.to_string()),
                            },
                        );
                    }
                }
            }
        }

        // Find all the AUX files
        for (version, kernel) in kernels.iter_mut() {
            let variant_str = kernel.variant.as_ref().map(|v| format!(".{}", v)).unwrap_or_default();
            let sysmap_file = format!("System.map-{}{}", version, variant_str);
            let cmdline_file = format!("cmdline-{}{}", version, variant_str);
            let config_file = format!("config-{}{}", version, variant_str);
            let initrd_file = format!(
                "initrd-{}{}{}",
                namespace,
                kernel.variant.as_ref().map(|v| format!(".{}.", v)).unwrap_or_default(),
                version
            );

            if let Some(p) = paths.iter().find(|p| p.as_ref().ends_with(&sysmap_file)) {
                kernel.extras.push(AuxilliaryFile {
                    path: p.as_ref().into(),
                    kind: AuxilliaryKind::SystemMap,
                });
            }
            if let Some(p) = paths.iter().find(|p| p.as_ref().ends_with(&cmdline_file)) {
                kernel.extras.push(AuxilliaryFile {
                    path: p.as_ref().into(),
                    kind: AuxilliaryKind::Cmdline,
                });
            }
            if let Some(p) = paths.iter().find(|p| p.as_ref().ends_with(&config_file)) {
                kernel.extras.push(AuxilliaryFile {
                    path: p.as_ref().into(),
                    kind: AuxilliaryKind::Config,
                });
            }
            if let Some(p) = paths.iter().find(|p| p.as_ref().ends_with(&initrd_file)) {
                kernel.initrd.push(AuxilliaryFile {
                    path: p.as_ref().into(),
                    kind: AuxilliaryKind::InitRD,
                });
            }
        }
        kernels.into_values().collect::<Vec<_>>()
    }

    // Handle newstyle discovery
    // TODO: Implement
    fn blsforme_kernels(paths: impl Iterator<Item = impl AsRef<Path>>) -> Vec<Kernel> {
        unimplemented!()
    }
}
