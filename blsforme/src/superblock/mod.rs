// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Superblock detection for various filesystems

use std::io::{self, Read, Seek};

use thiserror::Error;

pub mod btrfs;
pub mod ext4;
pub mod f2fs;

/// Encapsulate all supported superblocks
/// TODO: Re-evaluate use of Box when all sizes are similar.
#[derive(Debug)]
pub enum Superblock {
    BTRFS(Box<btrfs::Superblock>),
    Ext4(Box<ext4::Superblock>),
    F2FS(Box<f2fs::Superblock>),
}

impl Superblock {
    /// Filesystem UUID
    pub fn uuid(&self) -> String {
        match &self {
            Superblock::BTRFS(block) => block.uuid(),
            Superblock::Ext4(block) => block.uuid(),
            Superblock::F2FS(block) => block.uuid(),
        }
    }

    /// Volume label
    pub fn label(&self) -> Result<String, Error> {
        match &self {
            Superblock::BTRFS(_block) => Err(self::Error::UnsupportedFeature),
            Superblock::Ext4(block) => Ok(block.label()?),
            Superblock::F2FS(block) => Ok(block.label()?),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown superblock")]
    UnknownSuperblock,

    // ie label requests on partially implemented superblocks
    #[error("unsupported feature")]
    UnsupportedFeature,

    #[error("ext4: {0}")]
    EXT4(#[from] ext4::Error),

    #[error("btrfs: {0}")]
    BTRFS(#[from] btrfs::Error),

    #[error("f2fs: {0}")]
    F2FS(#[from] f2fs::Error),

    #[error("io: {0}")]
    IO(#[from] io::Error),
}

/// Attempt to find a superblock decoder for the given reader
pub fn for_reader<R: Read + Seek>(reader: &mut R) -> Result<Superblock, Error> {
    reader.rewind()?;

    // try ext4
    if let Ok(block) = ext4::Superblock::from_reader(reader) {
        return Ok(Superblock::Ext4(Box::new(block)));
    }

    // try btrfs now
    reader.rewind()?;
    if let Ok(block) = btrfs::Superblock::from_reader(reader) {
        return Ok(Superblock::BTRFS(Box::new(block)));
    }

    // try f2fs
    reader.rewind()?;
    if let Ok(block) = f2fs::Superblock::from_reader(reader) {
        return Ok(Superblock::F2FS(Box::new(block)));
    }

    Err(Error::UnknownSuperblock)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Cursor, Read},
    };

    use crate::superblock::Superblock;

    use super::for_reader;

    #[test]
    fn test_determination() {
        // Swings and roundabouts: Unpack ztd ext4 image in memory to get the Seekable trait we need
        // While each Superblock API is non-seekable, we enforce superblock::for_reader to be seekable
        // to make sure we pre-read a blob and pass it in for rewind/speed.
        let mut fi =
            fs::File::open("../test/blocks/ext4.img.zst").expect("Cannot find ext4 test image");
        let mut stream = zstd::stream::Decoder::new(&mut fi).expect("Unable to decode stream");
        // Roughly 6mib unpack target needed
        let mut memory = Vec::with_capacity(6 * 1024 * 1024);
        stream
            .read_to_end(&mut memory)
            .expect("Could not unpack ext4 filesystem in memory");

        let mut cursor = Cursor::new(&mut memory);
        let block = for_reader(&mut cursor).expect("Failed to find right block implementation");
        assert!(matches!(block, Superblock::Ext4(_)));
    }
}
