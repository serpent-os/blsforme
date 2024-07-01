// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Kernel abstraction

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::Error;

/// Control kernel discovery mechanism
#[derive(Debug)]
pub enum Schema {
    /// Legacy (clr-boot-manager style) schema
    Legacy(&'static str),

    /// Modern schema (actually has a schema.)
    Blsforme,
}

/// `boot.json` deserialise support
#[derive(Deserialize)]
pub struct BootJSON<'a> {
    /// Kernel's package name
    #[serde(borrow)]
    name: &'a str,

    /// Kernel's version string (uname -r)
    #[serde(borrow)]
    version: &'a str,

    /// Kernel's variant id
    #[serde(borrow)]
    variant: &'a str,
}

/// A kernel is the primary bootable element that we care about, ie
/// the vmlinuz file. It also comes with a set of auxilliary files
/// that are required for a fully working system, but specifically
/// dependent on that kernel version.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
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
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum AuxilliaryKind {
    /// A cmdline snippet
    Cmdline,

    /// An initial ramdisk
    InitRD,

    /// System.map file
    SystemMap,

    /// .config file
    Config,

    /// The `boot.json` file
    BootJSON,
}

/// An additional file required to be shipped with the kernel,
/// such as initrds, system maps, etc.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct AuxilliaryFile {
    pub path: PathBuf,
    pub kind: AuxilliaryKind,
}

impl Schema {
    /// Given a set of kernel-like paths, yield all potential kernels within them
    /// This should be a set of `/usr/lib/kernel` paths. Use glob or appropriate to discover.
    pub fn discover_system_kernels(&self, paths: impl Iterator<Item = impl AsRef<Path>>) -> Result<Vec<Kernel>, Error> {
        match &self {
            Schema::Legacy(name) => Ok(Self::legacy_kernels(name, paths)),
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

            kernel
                .initrd
                .sort_by_key(|i| i.path.display().to_string().to_lowercase());
            kernel
                .extras
                .sort_by_key(|e| e.path.display().to_string().to_lowercase());
        }
        kernels.into_values().collect::<Vec<_>>()
    }

    // Handle newstyle discovery
    fn blsforme_kernels(paths: impl Iterator<Item = impl AsRef<Path>>) -> Result<Vec<Kernel>, Error> {
        let all_paths = paths.map(|m| m.as_ref().to_path_buf()).collect::<BTreeSet<_>>();

        // all `vmlinuz` files within the set
        let mut kernel_images = all_paths
            .iter()
            .filter(|p| p.ends_with("vmlinuz"))
            .filter_map(|m| {
                let version = m.parent()?.file_name()?.to_str()?.to_string();
                Some((
                    version.clone(),
                    Kernel {
                        version,
                        image: PathBuf::from(m),
                        initrd: vec![],
                        extras: vec![],
                        variant: None,
                    },
                ))
            })
            .collect::<HashMap<_, _>>();

        // Walk kernels, find matching assets
        for (version, kernel) in kernel_images.iter_mut() {
            let lepath = kernel
                .image
                .parent()
                .ok_or_else(|| Error::InvalidFilesystem)?
                .to_str()
                .ok_or_else(|| Error::InvalidFilesystem)?;
            let versioned_assets = all_paths
                .iter()
                .filter(|p| !p.ends_with("vmlinuz") && p.starts_with(lepath) && !p.ends_with(version));
            for asset in versioned_assets {
                let filename = asset
                    .file_name()
                    .ok_or_else(|| Error::InvalidFilesystem)?
                    .to_str()
                    .ok_or_else(|| Error::InvalidFilesystem)?;
                let aux = match filename {
                    "System.map" => Some(AuxilliaryFile {
                        path: asset.clone(),
                        kind: AuxilliaryKind::SystemMap,
                    }),
                    "boot.json" => Some(AuxilliaryFile {
                        path: asset.clone(),
                        kind: AuxilliaryKind::BootJSON,
                    }),
                    "config" => Some(AuxilliaryFile {
                        path: asset.clone(),
                        kind: AuxilliaryKind::Config,
                    }),
                    _ if filename.ends_with(".initrd") => Some(AuxilliaryFile {
                        path: asset.clone(),
                        kind: AuxilliaryKind::InitRD,
                    }),
                    _ if filename.ends_with(".cmdline") => Some(AuxilliaryFile {
                        path: asset.clone(),
                        kind: AuxilliaryKind::Cmdline,
                    }),
                    _ => None,
                };

                if let Some(aux_file) = aux {
                    if matches!(aux_file.kind, AuxilliaryKind::InitRD) {
                        kernel.initrd.push(aux_file);
                    } else {
                        kernel.extras.push(aux_file);
                    }
                }

                kernel
                    .initrd
                    .sort_by_key(|i| i.path.display().to_string().to_lowercase());
                kernel
                    .extras
                    .sort_by_key(|e| e.path.display().to_string().to_lowercase());
            }
        }

        Ok(kernel_images.into_values().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::BootJSON;

    #[test]
    fn test_boot_json() {
        let text = fs::read_to_string("boot.json").expect("Failed to read json file");
        let boot = serde_json::from_str::<BootJSON>(&text).expect("Failed to decode JSON");
        assert_eq!(boot.name, "linux-desktop");
        assert_eq!(boot.variant, "desktop");
        assert_eq!(boot.version, "6.8.2-25.desktop");
    }
}
