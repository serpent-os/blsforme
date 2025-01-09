// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! F2FS superblock handling

use crate::{Error, Kind, Superblock};
use std::{
    io::{self, Read},
    ptr, slice,
};
use uuid::Uuid;

// Constants to allow us to move away from unsafe{} APIs
// in future, i.e. read_array(MAX_EXTENSION) ...

const MAX_VOLUME_LEN: usize = 512;
const MAX_EXTENSION: usize = 64;
const EXTENSION_LEN: usize = 8;
const VERSION_LEN: usize = 256;
const MAX_DEVICES: usize = 8;
const MAX_QUOTAS: usize = 3;
const MAX_STOP_REASON: usize = 32;
const MAX_ERRORS: usize = 16;

#[derive(Debug)]
#[repr(C, packed)]
pub struct F2FS {
    magic: u32,
    major_ver: u16,
    minor_ver: u16,
    log_sectorsize: u32,
    log_sectors_per_block: u32,
    log_blocksize: u32,
    log_blocks_per_seg: u32,
    segs_per_sec: u32,
    secs_per_zone: u32,
    checksum_offset: u32,
    block_count: u64,
    section_count: u32,
    segment_count: u32,
    segment_count_ckpt: u32,
    segment_count_sit: u32,
    segment_count_nat: u32,
    segment_count_ssa: u32,
    segment_count_main: u32,
    segment0_blkaddr: u32,
    cp_blkaddr: u32,
    sit_blkaddr: u32,
    nat_blkaddr: u32,
    ssa_blkaddr: u32,
    main_blkaddr: u32,
    root_ino: u32,
    node_ino: u32,
    meta_ino: u32,
    uuid: [u8; 16],
    volume_name: [u16; MAX_VOLUME_LEN],
    extension_count: u32,
    extension_list: [[u8; EXTENSION_LEN]; MAX_EXTENSION],
    cp_payload: u32,
    version: [u8; VERSION_LEN],
    init_version: [u8; VERSION_LEN],
    feature: u32,
    encryption_level: u8,
    encryption_pw_salt: [u8; 16],
    devs: [Device; MAX_DEVICES],
    qf_ino: [u32; MAX_QUOTAS],
    hot_ext_count: u8,
    s_encoding: u16,
    s_encoding_flags: u16,
    s_stop_reason: [u8; MAX_STOP_REASON],
    s_errors: [u8; MAX_ERRORS],
    reserved: [u8; 258],
    crc: u32,
}

/// struct f2fs_device
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Device {
    path: [u8; 64],
    total_segments: u32,
}

const MAGIC: u32 = 0xF2F52010;
const START_POSITION: u64 = 1024;

/// Attempt to decode the Superblock from the given read stream
pub fn from_reader<R: Read>(reader: &mut R) -> Result<F2FS, Error> {
    const SIZE: usize = std::mem::size_of::<F2FS>();
    let mut data: F2FS = unsafe { std::mem::zeroed() };
    let data_sliced = unsafe { slice::from_raw_parts_mut(&mut data as *mut _ as *mut u8, SIZE) };

    // Drop unwanted bytes (Seek not possible with zstd streamed inputs)
    io::copy(&mut reader.by_ref().take(START_POSITION), &mut io::sink())?;
    reader.read_exact(data_sliced)?;

    if data.magic != MAGIC {
        Err(Error::InvalidMagic)
    } else {
        log::trace!(
            "valid magic field: UUID={} [volume label: \"{}\"]",
            data.uuid()?,
            data.label().unwrap_or_else(|_| "[invalid utf8]".into())
        );
        Ok(data)
    }
}

impl Superblock for F2FS {
    /// Return the encoded UUID for this superblock
    fn uuid(&self) -> Result<String, Error> {
        Ok(Uuid::from_bytes(self.uuid).hyphenated().to_string())
    }

    /// Return the volume label as valid utf16 String
    fn label(&self) -> Result<String, Error> {
        let vol = unsafe { ptr::read_unaligned(ptr::addr_of!(self.volume_name)) };
        let prelim_label = String::from_utf16(&vol)?;
        // Need valid grapheme step and skip (u16)\0 nul termination in fixed block size
        Ok(prelim_label.trim_end_matches('\0').to_owned())
    }

    fn kind(&self) -> Kind {
        Kind::F2FS
    }
}

#[cfg(test)]
mod tests {

    use crate::{f2fs::from_reader, Superblock};
    use std::fs;

    #[test]
    fn test_basic() {
        let mut fi = fs::File::open("tests/f2fs.img.zst").expect("cannot open f2fs img");
        let mut stream = zstd::stream::Decoder::new(&mut fi).expect("Unable to decode stream");
        let sb = from_reader(&mut stream).expect("Cannot parse superblock");
        let label = sb.label().expect("Cannot determine volume name");
        assert_eq!(label, "blsforme testing");
        assert_eq!(sb.uuid().unwrap(), "d2c85810-4e75-4274-bc7d-a78267af7443");
    }
}
