// SPDX-FileCopyrightText: Copyright © 2024 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! XFS superblock handling

use std::{io::Read, slice};

use uuid::Uuid;

use super::{Error, Superblock};

// XFS typedefs
type RfsBlock = u64;
type RtbXlen = u64;
type FsBlock = u64;
type Ino = i64;
type AgBlock = u32;
type AgCount = u32;
type ExtLen = u32;
type Lsn = i64;

const MAX_LABEL_LEN: usize = 12;

/// XFS superblock, aligned to 64-bit
/// Note: Multi-byte integers (>{i,u}8) must be read as Big Endian
#[derive(Debug)]
#[repr(C, align(8))]
pub struct XFS {
    magicnum: u32,
    blocksize: u32,
    dblocks: RfsBlock,
    rblocks: RfsBlock,
    rextents: RtbXlen,
    uuid: [u8; 16],
    logstart: FsBlock,
    rootino: Ino,
    rbmino: Ino,
    rsumino: Ino,
    rextsize: AgBlock,
    agblocks: AgBlock,
    agcount: AgCount,
    rbmblocks: ExtLen,
    logblocks: ExtLen,
    versionnum: u16,
    sectsize: u16,
    inodesize: u16,
    inopblock: u16,
    fname: [u8; MAX_LABEL_LEN],
    blocklog: u8,
    sectlog: u8,
    inodelog: u8,
    inopblog: u8,
    agblklog: u8,
    rextslog: u8,
    inprogress: u8,
    imax_pct: u8,

    icount: u64,
    ifree: u64,
    fdblocks: u64,
    frextents: u64,

    uquotino: Ino,
    gquotino: Ino,
    qflags: u16,
    flags: u8,
    shared_vn: u8,
    inoalignment: ExtLen,
    unit: u32,
    width: u32,
    dirblklog: u8,
    logsectlog: u8,
    logsectsize: u16,
    logsunit: u32,
    features2: u32,

    bad_features: u32,

    features_compat: u32,
    features_ro_cmopat: u32,
    features_incompat: u32,
    features_log_incompat: u32,

    crc: u32,
    spino_align: ExtLen,

    pquotino: Ino,
    lsn: Lsn,
    meta_uuid: [u8; 16],
}

/// Magic = 'XFSB'
const MAGIC: u32 = 0x58465342;

/// Attempt to decode the Superblock from the given read stream
pub fn from_reader<R: Read>(reader: &mut R) -> Result<XFS, Error> {
    const SIZE: usize = std::mem::size_of::<XFS>();
    let mut data: XFS = unsafe { std::mem::zeroed() };
    let data_sliced = unsafe { slice::from_raw_parts_mut(&mut data as *mut _ as *mut u8, SIZE) };
    reader.read_exact(data_sliced)?;

    // Force big-endian
    data.magicnum = data.magicnum.to_be();

    if data.magicnum != MAGIC {
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

impl Superblock for XFS {
    fn kind(&self) -> super::Kind {
        super::Kind::XFS
    }

    /// Return `uuid` as a properly formatted 128-bit UUID
    fn uuid(&self) -> Result<String, super::Error> {
        Ok(Uuid::from_bytes(self.uuid).hyphenated().to_string())
    }

    /// Return `fname` (volume name) as utf8 string
    fn label(&self) -> Result<String, super::Error> {
        Ok(std::str::from_utf8(&self.fname)?
            .trim_end_matches('\0')
            .to_owned())
    }
}

#[cfg(test)]
mod tests {

    use crate::superblock::{xfs::from_reader, Superblock};
    use std::fs;

    #[test]
    fn test_basic() {
        let mut fi = fs::File::open("../test/blocks/xfs.img.zst").expect("cannot open xfs img");
        let mut stream = zstd::stream::Decoder::new(&mut fi).expect("Unable to decode stream");
        let sb = from_reader(&mut stream).expect("Cannot parse superblock");
        let label = sb.label().expect("Cannot determine volume name");
        assert_eq!(label, "BLSFORME");
        assert_eq!(sb.uuid().unwrap(), "45e8a3bf-8114-400f-95b0-380d0fb7d42d");
    }
}
