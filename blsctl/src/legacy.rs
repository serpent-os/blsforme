// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Support for legacy schema (CBM days) kernel packaging

use std::{fs, path::Path};

/// A discovery type: Will not be public forever.
#[derive(Debug)]
pub struct LegacyKernel {
    version: String,
    variant: String,
    release: u64,
}

/// Why oh why did we invent this poxy scheme
pub fn discover_kernels_legacy(
    namespace: &str,
    root: impl AsRef<Path>,
) -> color_eyre::Result<Vec<LegacyKernel>> {
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
                    version: version.into(),
                    variant: variant.into(),
                    release: release.chars().skip(1).collect::<String>().parse::<u64>()?,
                })
            }
        }
    }

    // reverse relno-sorted kernel set
    initial_kernels.sort_by_key(|k| k.release);
    initial_kernels.reverse();

    for kernel in initial_kernels.iter() {
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
        }
        if initrd.exists() {
            log::debug!("initrd: {initrd:?}");
        }
        if config.exists() {
            log::debug!("config: {config:?}");
        }
        if sysmap.exists() {
            log::debug!("sysmap: {sysmap:?}");
        }
    }

    Ok(initial_kernels)
}
