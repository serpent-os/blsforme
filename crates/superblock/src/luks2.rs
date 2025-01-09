// SPDX-FileCopyrightText: Copyright © 2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! LUKS2 superblock support

use crate::{Error, Kind, Superblock};
use log;
use std::{io::Read, ptr, slice};

const MAGIC_LEN: usize = 6;
const LABEL_LEN: usize = 48;
const CHECKSUM_ALG_LEN: usize = 32;
const SALT_LEN: usize = 64;
const UUID_LEN: usize = 40;
const CHECKSUM_LEN: usize = 64;

/// Per the `cryptsetup` docs for dm-crypt backed LUKS2, header is at first byte.
#[derive(Debug)]
#[repr(C, packed)]
pub struct Luks2 {
    magic: [u8; MAGIC_LEN],
    version: u16,
    hdr_size: u64,
    seqid: u64,
    label: [u8; LABEL_LEN],
    checksum_alg: [u8; CHECKSUM_ALG_LEN],
    salt: [u8; SALT_LEN],
    uuid: [u8; UUID_LEN],
    subsystem: [u8; LABEL_LEN],
    hdr_offset: u64,
    padding: [u8; 184],
    csum: [u8; CHECKSUM_LEN],
    padding4096: [u8; 7 * 512],
}

// Magic matchers: Guessing someone fudged endian encoding at some point.
const MAGIC1: [u8; MAGIC_LEN] = [b'L', b'U', b'K', b'S', 0xba, 0xbe];
const MAGIC2: [u8; MAGIC_LEN] = [b'S', b'K', b'U', b'L', 0xba, 0xbe];

/// Attempt to decode the Superblock from the given read stream
pub fn from_reader<R: Read>(reader: &mut R) -> Result<Luks2, Error> {
    const SIZE: usize = std::mem::size_of::<Luks2>();
    let mut data: Luks2 = unsafe { std::mem::zeroed() };
    let data_sliced = unsafe { slice::from_raw_parts_mut(&mut data as *mut _ as *mut u8, SIZE) };
    reader.read_exact(data_sliced)?;

    let magic = unsafe { ptr::read_unaligned(ptr::addr_of!(data.magic)) };

    match magic {
        MAGIC1 | MAGIC2 => {
            log::trace!(
                "valid magic field: UUID={} [volume label: \"{}\"]",
                data.uuid()?,
                data.label().unwrap_or_else(|_| "[invalid utf8]".into())
            );
            Ok(data)
        }
        _ => Err(Error::InvalidMagic),
    }
}

impl Superblock for Luks2 {
    fn kind(&self) -> Kind {
        Kind::LUKS2
    }

    /// NOTE: LUKS2 stores string UUID rather than 128-bit sequence..
    fn uuid(&self) -> Result<String, super::Error> {
        let uuid = unsafe { ptr::read_unaligned(ptr::addr_of!(self.uuid)) };
        Ok(std::str::from_utf8(&uuid)?.trim_end_matches('\0').to_owned())
    }

    /// NOTE: Label is often empty, set in config instead...
    fn label(&self) -> Result<String, super::Error> {
        let label = unsafe { ptr::read_unaligned(ptr::addr_of!(self.label)) };
        Ok(std::str::from_utf8(&label)?.trim_end_matches('\0').to_owned())
    }
}

#[cfg(test)]
mod tests {

    use crate::{luks2::from_reader, Superblock};
    use std::fs;

    #[test]
    fn test_basic() {
        let mut fi = fs::File::open("tests/luks+ext4.img.zst").expect("cannot open luks2 img");
        let mut stream = zstd::stream::Decoder::new(&mut fi).expect("Unable to decode stream");
        let sb = from_reader(&mut stream).expect("Cannot parse superblock");
        assert_eq!(sb.uuid().unwrap(), "be373cae-2bd1-4ad5-953f-3463b2e53e59");
    }
}
