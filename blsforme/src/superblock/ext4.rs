// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! EXT4 superblock handling

use core::slice;
use std::io::{self, Read, Seek};

use thiserror::Error;
use uuid::Uuid;

/// EXT4 Superblock definition (as seen in the kernel)
#[derive(Debug)]
#[repr(C)]
pub struct Superblock {
    inodes_count: u32,
    block_counts_lo: u32,
    r_blocks_count_lo: u32,
    free_blocks_count_lo: u32,
    free_inodes_count: u32,
    first_data_block: u32,
    log_block_size: u32,
    log_cluster_size: u32,
    blocks_per_group: u32,
    clusters_per_group: u32,
    inodes_per_group: u32,
    m_time: u32,
    w_time: u32,
    mnt_count: u16,
    max_mnt_count: u16,
    magic: u16,
    state: u16,
    errors: u16,
    minor_rev_level: u16,
    lastcheck: u32,
    checkinterval: u32,
    creator_os: u32,
    rev_level: u32,
    def_resuid: u16,
    def_resgid: u16,
    first_ino: u32,
    inode_size: u16,
    block_group_nr: u16,
    feature_compat: u32,
    feature_incompat: u32,
    feature_ro_compat: u32,
    uuid: [u8; 16],
    volume_name: [u8; 16],
    last_mounted: [u8; 64],
    algorithm_usage_bitmap: u32,
    prealloc_blocks: u8,
    prealloc_dir_blocks: u8,
    reserved_gdt_blocks: u16,
    journal_uuid: [u8; 16],
    journal_inum: u32,
    journal_dev: u32,
    last_orphan: u32,
    hash_seed: [u32; 4],
    def_hash_version: u8,
    jnl_backup_type: u8,
    desc_size: u16,
    default_mount_opts: u32,
    first_meta_bg: u32,
    mkfs_time: u32,
    jnl_blocks: [u32; 17],
    blocks_count_hi: u32,
    free_blocks_count_hi: u32,
    min_extra_isize: u16,
    want_extra_isize: u16,
    flags: u32,
    raid_stride: u16,
    mmp_update_interval: u16,
    mmp_block: u64,
    raid_stripe_width: u32,
    log_groups_per_flex: u8,
    checksum_type: u8,
    reserved_pad: u16,
    kbytes_written: u64,
    snapshot_inum: u32,
    snapshot_id: u32,
    snapshot_r_blocks_count: u64,
    snapshot_list: u32,
    error_count: u32,
    first_error_time: u32,
    first_error_inod: u32,
    first_error_block: u64,
    first_error_func: [u8; 32],
    first_error_line: u32,
    last_error_time: u32,
    last_error_inod: u32,
    last_error_line: u32,
    last_error_block: u64,
    last_error_func: [u8; 32],
    mount_opts: [u8; 64],
    usr_quota_inum: u32,
    grp_quota_inum: u32,
    overhead_clusters: u32,
    reserved: [u32; 108],
    checksum: u32,
}

/// Ext4 specific decoding errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("not a valid ext source")]
    InvalidMagic,

    #[error("invalid utf8 in volume label: {0}")]
    InvalidLabel(#[from] std::str::Utf8Error),

    #[error("io error: {0}")]
    IO(#[from] io::Error),
}

const MAGIC: u16 = 0xEF53;

impl Superblock {
    /// Attempt to decode the Superblock from the given read stream
    pub fn from_reader<R: Read + Seek>(mut reader: R) -> Result<Self, Error> {
        const SIZE: usize = std::mem::size_of::<Superblock>();
        let mut data: Superblock = unsafe { std::mem::zeroed() };
        let data_sliced =
            unsafe { slice::from_raw_parts_mut(&mut data as *mut _ as *mut u8, SIZE) };

        // Skip to 1024, read 1024 bytes
        reader.seek(std::io::SeekFrom::Start(1024))?;
        reader.read_exact(data_sliced)?;

        if data.magic != MAGIC {
            Err(Error::InvalidMagic)
        } else {
            log::trace!(
                "ext4: valid magic field for UUID={}, LABEL={}",
                data.uuid(),
                data.label().unwrap_or_else(|_| "[invalid utf8]".into())
            );
            Ok(data)
        }
    }

    /// Return the encoded UUID for this superblock
    pub fn uuid(&self) -> String {
        Uuid::from_bytes(self.uuid).hyphenated().to_string()
    }

    /// Return the volume label as valid utf8
    pub fn label(&self) -> Result<String, Error> {
        Ok(std::str::from_utf8(&self.volume_name)?.into())
    }
}

#[cfg(test)]
mod tests {

    use super::Superblock;
    use std::fs;

    #[test]
    fn test_basic() {
        let fi = fs::File::open("../test/blocks/ext4.img").expect("cannot open ext4 img");
        let sb = Superblock::from_reader(fi).expect("Cannot parse superblock");
        let label = sb.label().expect("Cannot determine volume name");
        assert_eq!(label, "blsforme testing");
        assert_eq!(sb.uuid(), "731af94c-9990-4eed-944d-5d230dbe8a0d");
    }
}
