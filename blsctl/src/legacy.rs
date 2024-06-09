// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Support for legacy schema (CBM days) kernel packaging

use std::{
    fs,
    path::{Path, PathBuf},
};

use blsforme::kernel::{AuxilliaryFile, AuxilliaryKind, Kernel};

/// A discovery type: Will not be public forever.
#[derive(Debug)]
struct LegacyKernel {
    version: String,
    path: PathBuf,
    variant: String,
    release: u64,
    aux: Vec<AuxilliaryFile>,
}

impl From<LegacyKernel> for Kernel {
    fn from(val: LegacyKernel) -> Self {
        let (initrd, extras) = val
            .aux
            .into_iter()
            .partition(|a| matches!(a.kind, AuxilliaryKind::InitRD));
        Kernel {
            version: val.version,
            image: val.path,
            initrd,
            extras,
        }
    }
}

/// Why oh why did we invent this poxy scheme
pub fn discover_kernels_legacy(namespace: &str, root: impl AsRef<Path>) -> color_eyre::Result<Vec<Kernel>> {
    let root = root.as_ref().join("usr").join("lib").join("kernel");
    let mut initial_kernels = vec![];

    for pair in fs::read_dir(&root)? {
        let item = pair?;
        let file_name = item.file_name().to_string_lossy().to_string();

        if !file_name.starts_with(namespace) || file_name.len() < namespace.len() + 1 {
            continue;
        }

        let (left, right) = file_name.split_at(namespace.len() + 1);
        assert!(left.ends_with('.'));
        if let Some((variant, version)) = right.split_once('.') {
            if let Some((version, release)) = version.rfind('-').map(|i| version.split_at(i)) {
                log::trace!("discovered vmlinuz: {file_name}");
                initial_kernels.push(LegacyKernel {
                    path: item.path(),
                    version: version.into(),
                    variant: variant.into(),
                    release: release.chars().skip(1).collect::<String>().parse::<u64>()?,
                    aux: vec![],
                })
            }
        }
    }

    // reverse relno-sorted kernel set
    initial_kernels.sort_by_key(|k| k.release);
    initial_kernels.reverse();

    for kernel in initial_kernels.iter_mut() {
        let cmdline = root.join(format!(
            "cmdline-{}-{}.{}",
            &kernel.version, kernel.release, &kernel.variant
        ));
        let initrd = root.join(format!(
            "initrd-{}.{}.{}-{}",
            namespace, &kernel.variant, &kernel.version, kernel.release
        ));
        let config = root.join(format!(
            "config-{}-{}.{}",
            &kernel.version, kernel.release, &kernel.variant
        ));
        let sysmap = root.join(format!(
            "System.map-{}-{}.{}",
            &kernel.version, kernel.release, &kernel.variant
        ));

        if cmdline.exists() {
            log::debug!("cmdline: {cmdline:?}");
            kernel.aux.push(AuxilliaryFile {
                path: cmdline,
                kind: AuxilliaryKind::Cmdline,
            })
        }
        if initrd.exists() {
            log::debug!("initrd: {initrd:?}");
            kernel.aux.push(AuxilliaryFile {
                path: initrd,
                kind: AuxilliaryKind::InitRD,
            })
        }
        if config.exists() {
            log::debug!("config: {config:?}");
            kernel.aux.push(AuxilliaryFile {
                path: config,
                kind: AuxilliaryKind::Config,
            })
        }
        if sysmap.exists() {
            log::debug!("sysmap: {sysmap:?}");
            kernel.aux.push(AuxilliaryFile {
                path: sysmap,
                kind: AuxilliaryKind::SystemMap,
            })
        }
    }

    Ok(initial_kernels.into_iter().map(|m| m.into()).collect())
}
