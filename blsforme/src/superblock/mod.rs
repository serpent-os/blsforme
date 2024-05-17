// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Superblock detection for various filesystems

use std::io::{Read, Seek};

use thiserror::Error;

pub mod btrfs;
pub mod ext4;

/// Encapsulate all supported superblocks
/// TODO: Re-evaluate use of Box when all sizes are similar.
#[derive(Debug)]
pub enum Superblock {
    BTRFS(Box<btrfs::Superblock>),
    Ext4(Box<ext4::Superblock>),
}

impl Superblock {
    /// Filesystem UUID
    pub fn uuid(&self) -> String {
        match &self {
            Superblock::BTRFS(block) => block.uuid(),
            Superblock::Ext4(block) => block.uuid(),
        }
    }

    /// Volume label
    pub fn label(&self) -> Result<String, Error> {
        match &self {
            Superblock::BTRFS(_block) => Err(self::Error::UnsupportedFeature),
            Superblock::Ext4(block) => Ok(block.label()?),
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
}

/// Attempt to find a superblock decoder for the given reader
pub fn superblock_for_reader<R: Read + Seek>(reader: &mut R) -> Result<Superblock, Error> {
    if let Ok(block) = ext4::Superblock::from_reader(reader) {
        Ok(Superblock::Ext4(Box::new(block)))
    } else if let Ok(block) = btrfs::Superblock::from_reader(reader) {
        Ok(Superblock::BTRFS(Box::new(block)))
    } else {
        Err(Error::UnknownSuperblock)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::superblock::Superblock;

    use super::superblock_for_reader;

    #[test]
    fn test_determination() {
        let mut fi =
            fs::File::open("../test/blocks/ext4.img").expect("Cannot find ext4 test image");
        let block =
            superblock_for_reader(&mut fi).expect("Failed to find right block implementation");
        assert!(matches!(block, Superblock::Ext4(_)));
    }
}
