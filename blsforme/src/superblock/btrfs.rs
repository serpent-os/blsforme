// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! BTRFS superblock handling
//! TODO: Add full representation of the superblock to allow us to
//! discover volumes and the root label.

use core::slice;
use std::io::{self, Read, Seek};

use thiserror::Error;
use uuid::Uuid;

/// BTRFS superblock definition (as seen in the kernel)
/// This is a PARTIAL representation that matches only the
/// first 72 bytes, verifies the magic, and permits extraction
/// of the UUID
#[derive(Debug)]
#[repr(C)]
pub struct Superblock {
    csum: [u8; 32],
    fsid: [u8; 16],
    bytenr: u64,
    flags: u64,
    magic: u64,
    generation: u64,
    root: u64,
    chunk_root: u64,
    log_root: u64,
}

/// btrfs specific decoding errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("not a valid btrfs source")]
    InvalidMagic,

    #[error("io error: {0}")]
    IO(#[from] io::Error),
}

// Superblock starts at 65536 for btrfs.
const START_POSITION: u64 = 0x10000;

// "_BHRfS_M"
const MAGIC: u64 = 0x4D5F53665248425F;

impl Superblock {
    /// Attempt to decode the Superblock from the given read stream
    pub fn from_reader<R: Read + Seek>(reader: &mut R) -> Result<Self, Error> {
        const SIZE: usize = std::mem::size_of::<Superblock>();
        let mut data: Superblock = unsafe { std::mem::zeroed() };
        let data_sliced =
            unsafe { slice::from_raw_parts_mut(&mut data as *mut _ as *mut u8, SIZE) };

        // Skip to start
        reader.seek(std::io::SeekFrom::Start(START_POSITION))?;
        reader.read_exact(data_sliced)?;

        if data.magic != MAGIC {
            Err(Error::InvalidMagic)
        } else {
            log::trace!("valid magic field: UUID={}", data.uuid());
            Ok(data)
        }
    }

    /// Return the encoded UUID for this superblock
    pub fn uuid(&self) -> String {
        Uuid::from_bytes(self.fsid).hyphenated().to_string()
    }
}

#[cfg(test)]
mod tests {

    use super::Superblock;
    use std::fs;

    #[test]
    fn test_basic() {
        let mut fi = fs::File::open("../test/blocks/btrfs.img").expect("cannot open ext4 img");
        let sb = Superblock::from_reader(&mut fi).expect("Cannot parse superblock");
        assert_eq!(sb.uuid(), "14df1be6-a2fb-45cd-9a1f-93d71c8e710f");
    }
}
