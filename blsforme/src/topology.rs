// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Query the topology of a target system

use std::{
    collections::HashMap,
    fs, io,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use nix::sys::stat;
use thiserror::Error;

use crate::{
    mtab::{self, MountOption},
    Configuration,
};

/// BIOS vs UEFI logic gating
#[derive(Debug)]
pub enum Firmware {
    BIOS,
    UEFI,
}

/// Basic errors in topology probe
#[derive(Debug, Error)]
pub enum Error {
    #[error("unsupported root filesystem")]
    UnsupportedRootFS,

    #[error("failed to decode partition id")]
    ParseInt(#[from] ParseIntError),

    #[error("no `mounts` entry for {0}")]
    UnknownMount(PathBuf),

    #[error("no partition {index} on device {device}")]
    UnknownPartition { device: String, index: u32 },

    #[error("io {0}")]
    IO(#[from] io::Error),

    #[error("lowlevel C stdlib error: {0}")]
    Errno(#[from] nix::errno::Errno),
}

/// Filesystems are passed by root=PARTUUID or root=UUID,
/// depending on whether GPT is in use (UUID for non GPT)
#[derive(Debug)]
pub enum FilesystemID {
    PartUUID(String),
    UUID(String),
    // Use device path
    Path(String),
}

impl FilesystemID {
    fn root_cmdline_partial(&self) -> String {
        match &self {
            FilesystemID::PartUUID(u) => format!("PARTUUID={u}"),
            FilesystemID::UUID(u) => format!("UUID={u}"),
            FilesystemID::Path(p) => p.clone(),
        }
    }
}

/// Nice wrapping of filesystems
#[derive(Debug)]
pub enum Filesystem {
    Btrfs {
        id: FilesystemID,
        subvol: Option<String>,
    },
    // Some identifier for a filesystem not needing specialisation
    Any(FilesystemID),
}

/// Encapsulation of a device characteristics
#[derive(Debug)]
pub struct BlockDevice {
    pub filesystem: Filesystem,
    pub path: PathBuf,
}

impl BlockDevice {
    /// Generate the root= rootfsflags= cmdline needed to utilise this block device
    pub fn root_cmdline(&self) -> String {
        match &self.filesystem {
            // TODO: Use UUID, account for LVM!
            Filesystem::Btrfs { subvol, id } => {
                if let Some(subvol) = subvol {
                    format!(
                        "root={} rootfsflags=subvol={}",
                        id.root_cmdline_partial(),
                        subvol
                    )
                } else {
                    format!("root={}", id.root_cmdline_partial())
                }
            }
            Filesystem::Any(id) => format!("root={}", id.root_cmdline_partial()),
        }
    }
}

/// The result of a topology probe
#[derive(Debug)]
pub struct Topology {
    /// Detected firmware
    pub firmware: Firmware,

    /// Results for the root filesystem
    pub rootfs: BlockDevice,
}

impl Topology {
    /// Return the probe result of a given configuration
    ///
    /// Note that UEFI detection is based solely upon the existence
    /// of `/sys/firmware/efi` being mounted inside the target (native OR image)
    ///
    /// As such, we expect bind-mounts in place for image-based modes to cooperate.
    ///
    /// Arguments:
    ///  - `config` - a [`crate::Configuration`]
    pub fn probe(config: &Configuration) -> Result<Self, self::Error> {
        let device = Self::get_device_for_root(config)?;
        Self::refine_device(config, device)
    }

    /// Attempt cascading discovery of the the rootfs block device
    fn get_device_for_root(config: &Configuration) -> Result<BlockDevice, self::Error> {
        match Self::get_device_by_mountpoint(config) {
            Ok(device) => Ok(device),
            Err(_) => {
                // TODO: Log error in mount discovery
                Ok(Self::get_device_by_stat(config)?)
            }
        }
    }

    // Process the global mount table and extrapolate a more detailed BlockDevice
    fn get_device_by_mountpoint(config: &Configuration) -> Result<BlockDevice, self::Error> {
        // Look up by mountpoint
        let table = mtab::MountTable::new_from_path("/proc/self/mounts")?;
        let mounts = table
            .iter()
            .map(|m| (PathBuf::from(m.mountpoint), m))
            .collect::<HashMap<_, _>>();

        let mount = mounts
            .get(config.root.path())
            .ok_or_else(|| Error::UnknownMount(config.root.path().clone()))?;
        // Map all key/value options in for easy access
        let options = mount
            .options()
            .filter_map(|m| {
                if let MountOption::Option(k, v) = m {
                    Some((k, v))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        let filesystem = match mount.filesystem {
            "btrfs" => Filesystem::Btrfs {
                id: FilesystemID::Path(mount.device.into()),
                subvol: options.get("subvol").map(|v| v.to_string()),
            },
            _ => Filesystem::Any(FilesystemID::Path(mount.device.into())),
        };
        Ok(BlockDevice {
            filesystem,
            path: mount.device.into(),
        })
    }

    /// Legacy approach to determination of block device by stat
    /// Total no go for btrfs
    fn get_device_by_stat(config: &Configuration) -> Result<BlockDevice, self::Error> {
        let st = stat::lstat(config.root.path())?;
        let major = stat::major(st.st_dev);
        let minor = stat::minor(st.st_dev);
        let path = config
            .root
            .path()
            .join("dev")
            .join("block")
            .join(format!("{major}:{minor}"));
        let fs_path = fs::canonicalize(path)?;
        Ok(BlockDevice {
            filesystem: Filesystem::Any(FilesystemID::Path(fs_path.to_string_lossy().to_string())),
            path: fs_path,
        })
    }

    /// Refine block device scan for further details
    ///
    /// At this point we have identified the root device, and can
    /// then scan the root disk to determine the PartUUID if applicable,
    /// if not we can fallback to superblock scanning.
    fn refine_device(config: &Configuration, device: BlockDevice) -> Result<Self, self::Error> {
        let efi_path = config.root.path().join("sys").join("firmware").join("efi");
        let firmware = if efi_path.exists() {
            Firmware::UEFI
        } else {
            Firmware::BIOS
        };

        let path = fs::canonicalize(device.path)?;
        let id = path
            .file_name()
            .ok_or_else(|| self::Error::UnknownMount(path.clone()))?;
        let sysfs_child = fs::canonicalize(
            config
                .root
                .path()
                .join("sys")
                .join("class")
                .join("block")
                .join(id),
        )?;

        // Grab the parent device name
        let sysfs_parent = sysfs_child
            .parent()
            .and_then(|p| p.file_name())
            .ok_or_else(|| self::Error::UnknownMount(sysfs_child.clone()))?;
        let parent_dev = config.root.path().join("dev").join(sysfs_parent);

        // Determine partition uuid
        let part_file = sysfs_child.join("partition");
        let fs_id = if part_file.exists() {
            let partition_index = fs::read_to_string(part_file)?.trim().parse::<u32>()?;
            match Self::scan_gpt_for_partuuid(parent_dev, partition_index) {
                Ok(pt_uuid) => Some(FilesystemID::PartUUID(pt_uuid)),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(Self {
            firmware,
            rootfs: BlockDevice {
                path,
                filesystem: Self::rewrap_filesystem(device.filesystem, fs_id),
            },
        })
    }

    /// Scan GPT partitions to gain PartUUID
    fn scan_gpt_for_partuuid(device: impl AsRef<Path>, index: u32) -> Result<String, Error> {
        let device = device.as_ref();
        let mut file = fs::File::open(device)?;
        let disk = gpt::GptConfig::new()
            .writable(false)
            .writable(false)
            .initialized(true)
            .open_from_device(Box::new(&mut file))?;

        let partition =
            disk.partitions()
                .get(&index)
                .ok_or_else(|| self::Error::UnknownPartition {
                    device: device.to_string_lossy().to_string(),
                    index,
                })?;
        Ok(partition.part_guid.hyphenated().to_string())
    }

    /// Rewrap Filesystem type with a new FilesystemID (PartUUID) for example
    fn rewrap_filesystem(fs: Filesystem, id: Option<FilesystemID>) -> Filesystem {
        if let Some(id) = id {
            match fs {
                Filesystem::Btrfs { subvol, .. } => Filesystem::Btrfs { id, subvol },
                Filesystem::Any(_) => Filesystem::Any(id),
            }
        } else {
            fs
        }
    }
}
