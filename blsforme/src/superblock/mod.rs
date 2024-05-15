// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! Superblock detection for various filesystems

use std::io::{Read, Seek};

use thiserror::Error;

pub mod ext4;

/// Encapsulate all supported superblocks
pub enum Superblock {
    Ext4(ext4::Superblock),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("unknown superblock")]
    UnknownSuperblock,
}

/// Attempt to find a superblock decoder for the given reader
pub fn superblock_for_reader<R: Read + Seek>(reader: R) -> Result<Superblock, Error> {
    if let Ok(block) = ext4::Superblock::from_reader(reader) {
        Ok(Superblock::Ext4(block))
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
        let fi = fs::File::open("../test/blocks/ext4.img").expect("Cannot find ext4 test image");
        let block = superblock_for_reader(fi).expect("Failed to find right block implementation");
        assert!(matches!(block, Superblock::Ext4(_)));
    }
}
