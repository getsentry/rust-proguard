use watto::Pod;

use super::{Error, ErrorKind};

/// The magic file preamble as individual bytes.
const PRGCACHE_MAGIC_BYTES: [u8; 4] = *b"PRGC";

/// The magic file preamble to identify PrgCache files.
///
/// Serialized as ASCII "PRGC" on little-endian (x64) systems.
pub(crate) const PRGCACHE_MAGIC: u32 = u32::from_le_bytes(PRGCACHE_MAGIC_BYTES);
/// The byte-flipped magic, which indicates an endianness mismatch.
pub(crate) const PRGCACHE_MAGIC_FLIPPED: u32 = PRGCACHE_MAGIC.swap_bytes();

pub const PRGCACHE_VERSION: u32 = 1;

/// The header of a proguard cache file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Header {
    /// The file magic representing the file format and endianness.
    pub(crate) magic: u32,
    /// The SymCache Format Version.
    pub(crate) version: u32,

    pub(crate) num_classes: u32,
    pub(crate) string_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Class {
    pub(crate) obfuscated_name_offset: u32,
    pub(crate) body_offset: u32,
}

unsafe impl Pod for Header {}
unsafe impl Pod for Class {}

/// The serialized ProguardCache binary format.
#[derive(Clone, PartialEq, Eq)]
pub struct ProguardCache<'data> {
    pub(crate) header: &'data Header,
    pub(crate) classes: &'data [Class],
    pub(crate) string_bytes: &'data [u8],
}

impl<'data> std::fmt::Debug for ProguardCache<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProguardCache")
            .field("version", &self.header.version)
            .field("classes", &self.header.num_classes)
            .field("string_bytes", &self.header.string_bytes)
            .finish()
    }
}

impl<'data> ProguardCache<'data> {
    pub fn parse(buf: &'data [u8]) -> Result<Self, Error> {
        let (header, rest) = Header::ref_from_prefix(buf).ok_or(ErrorKind::InvalidHeader)?;
        if header.magic == PRGCACHE_MAGIC_FLIPPED {
            return Err(ErrorKind::WrongEndianness.into());
        }
        if header.magic != PRGCACHE_MAGIC {
            return Err(ErrorKind::WrongFormat.into());
        }
        if header.version != PRGCACHE_VERSION {
            return Err(ErrorKind::WrongVersion.into());
        }

        let (_, rest) = watto::align_to(rest, 8).ok_or(ErrorKind::InvalidClasses)?;
        let (classes, rest) = Class::slice_from_prefix(rest, header.num_classes as usize)
            .ok_or(ErrorKind::InvalidClasses)?;

        let (_, string_bytes) =
            watto::align_to(rest, 8).ok_or(ErrorKind::UnexpectedStringBytes {
                expected: header.string_bytes as usize,
                found: 0,
            })?;
        if string_bytes.len() < header.string_bytes as usize {
            return Err(ErrorKind::UnexpectedStringBytes {
                expected: header.string_bytes as usize,
                found: string_bytes.len(),
            }
            .into());
        }

        Ok(Self {
            header,
            classes,
            string_bytes,
        })
    }
}
