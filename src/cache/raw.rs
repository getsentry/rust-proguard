use std::collections::BTreeMap;
use std::io::Write;

use watto::{Pod, StringTable};

use crate::builder::{self, ParsedProguardMapping};
use crate::ProguardMapping;

use super::{CacheError, CacheErrorKind};

/// The magic file preamble as individual bytes.
const PRGCACHE_MAGIC_BYTES: [u8; 4] = *b"PRGC";

/// The magic file preamble to identify ProguardCache files.
///
/// Serialized as ASCII "PRGC" on little-endian (x64) systems.
pub(crate) const PRGCACHE_MAGIC: u32 = u32::from_le_bytes(PRGCACHE_MAGIC_BYTES);
/// The byte-flipped magic, which indicates an endianness mismatch.
pub(crate) const PRGCACHE_MAGIC_FLIPPED: u32 = PRGCACHE_MAGIC.swap_bytes();

pub const PRGCACHE_VERSION: u32 = 2;

/// The header of a proguard cache file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Header {
    /// The file magic representing the file format and endianness.
    pub(crate) magic: u32,
    /// The ProguardCache Format Version.
    pub(crate) version: u32,
    /// The number of class entries in this cache.
    pub(crate) num_classes: u32,
    /// The total number of member entries in this cache.
    pub(crate) num_members: u32,
    /// The total number of member-by-params entries in this cache.
    pub(crate) num_members_by_params: u32,
    /// The number of string bytes in this cache.
    pub(crate) string_bytes: u32,
}

/// An entry for a class in a proguard cache file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Class {
    /// The obfuscated class name (offset into the string section).
    pub(crate) obfuscated_name_offset: u32,
    /// The original class name (offset into the string section).
    pub(crate) original_name_offset: u32,
    /// The file name (offset into the string section).
    pub(crate) file_name_offset: u32,
    /// The start of the class's member entries (offset into the member section).
    pub(crate) members_offset: u32,
    /// The number of member entries for this class.
    pub(crate) members_len: u32,
    /// The start of the class's member-by-params entries (offset into the member section).
    pub(crate) members_by_params_offset: u32,
    /// The number of member-by-params entries for this class.
    pub(crate) members_by_params_len: u32,
    /// Whether this class was synthesized by the compiler.
    ///
    /// `0` means `false`, all other values mean `true`.
    ///
    /// Note: It's currently unknown what effect a synthesized
    /// class has.
    pub(crate) is_synthesized: u8,

    /// Reserved space.
    pub(crate) _reserved: [u8; 3],
}

impl Class {
    /// Returns true if this class was synthesized by the compiler.
    pub(crate) fn is_synthesized(&self) -> bool {
        self.is_synthesized != 0
    }
}

impl Default for Class {
    fn default() -> Self {
        Self {
            obfuscated_name_offset: u32::MAX,
            original_name_offset: u32::MAX,
            file_name_offset: u32::MAX,
            members_offset: u32::MAX,
            members_len: 0,
            members_by_params_offset: u32::MAX,
            members_by_params_len: 0,
            is_synthesized: 0,
            _reserved: [0; 3],
        }
    }
}

/// An entry corresponding to a method line in a proguard cache file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[repr(C)]
pub(crate) struct Member {
    /// The obfuscated method name (offset into the string section).
    pub(crate) obfuscated_name_offset: u32,
    /// The start of the range covered by this entry (1-based).
    pub(crate) startline: u32,
    /// The end of the range covered by this entry (inclusive).
    pub(crate) endline: u32,
    /// The original class name (offset into the string section).
    pub(crate) original_class_offset: u32,
    /// The original file name (offset into the string section).
    pub(crate) original_file_offset: u32,
    /// The original method name (offset into the string section).
    pub(crate) original_name_offset: u32,
    /// The original start line (1-based).
    pub(crate) original_startline: u32,
    /// The original end line (inclusive).
    pub(crate) original_endline: u32,
    /// The entry's parameter string (offset into the strings section).
    pub(crate) params_offset: u32,
    /// Whether this member was synthesized by the compiler.
    ///
    /// `0` means `false`, all other values mean `true`.
    pub(crate) is_synthesized: u8,

    /// Reserved space.
    pub(crate) _reserved: [u8; 3],
}

impl Member {
    /// Returns true if this member was synthesized by the compiler.
    pub(crate) fn is_synthesized(&self) -> bool {
        self.is_synthesized != 0
    }
}

unsafe impl Pod for Header {}
unsafe impl Pod for Class {}
unsafe impl Pod for Member {}

/// The serialized `ProguardCache` binary format.
#[derive(Clone, PartialEq, Eq)]
pub struct ProguardCache<'data> {
    pub(crate) header: &'data Header,
    /// A list of class entries.
    ///
    /// Class entries are sorted by their obfuscated names.
    pub(crate) classes: &'data [Class],
    /// A list of member entries.
    ///
    /// Member entries are sorted by class, then
    /// obfuscated method name, and finally by the
    /// order in which they occurred in the original proguard file.
    pub(crate) members: &'data [Member],
    /// A list of member entries.
    ///
    /// These entries are sorted by class, then
    /// obfuscated method name, then params string.
    pub(crate) members_by_params: &'data [Member],
    /// The collection of all strings in the cache file.
    pub(crate) string_bytes: &'data [u8],
}

impl std::fmt::Debug for ProguardCache<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProguardCache")
            .field("version", &self.header.version)
            .field("classes", &self.header.num_classes)
            .field("members", &self.header.num_members)
            .field("members_by_params", &self.header.num_members_by_params)
            .field("string_bytes", &self.header.string_bytes)
            .finish()
    }
}

impl<'data> ProguardCache<'data> {
    /// Parses a `ProguardCache` out of bytes.
    pub fn parse(buf: &'data [u8]) -> Result<Self, CacheError> {
        let (header, rest) = Header::ref_from_prefix(buf).ok_or(CacheErrorKind::InvalidHeader)?;
        if header.magic == PRGCACHE_MAGIC_FLIPPED {
            return Err(CacheErrorKind::WrongEndianness.into());
        }
        if header.magic != PRGCACHE_MAGIC {
            return Err(CacheErrorKind::WrongFormat.into());
        }
        if header.version != PRGCACHE_VERSION {
            return Err(CacheErrorKind::WrongVersion.into());
        }

        let (_, rest) = watto::align_to(rest, 8).ok_or(CacheErrorKind::InvalidClasses)?;
        let (classes, rest) = Class::slice_from_prefix(rest, header.num_classes as usize)
            .ok_or(CacheErrorKind::InvalidClasses)?;

        let (_, rest) = watto::align_to(rest, 8).ok_or(CacheErrorKind::InvalidMembers)?;
        let (members, rest) = Member::slice_from_prefix(rest, header.num_members as usize)
            .ok_or(CacheErrorKind::InvalidMembers)?;

        let (_, rest) = watto::align_to(rest, 8).ok_or(CacheErrorKind::InvalidMembers)?;
        let (members_by_params, rest) =
            Member::slice_from_prefix(rest, header.num_members_by_params as usize)
                .ok_or(CacheErrorKind::InvalidMembers)?;

        let (_, string_bytes) =
            watto::align_to(rest, 8).ok_or(CacheErrorKind::UnexpectedStringBytes {
                expected: header.string_bytes as usize,
                found: 0,
            })?;

        if string_bytes.len() < header.string_bytes as usize {
            return Err(CacheErrorKind::UnexpectedStringBytes {
                expected: header.string_bytes as usize,
                found: string_bytes.len(),
            }
            .into());
        }

        Ok(Self {
            header,
            classes,
            members,
            members_by_params,
            string_bytes,
        })
    }

    /// Writes a [`ProguardMapping`] into a writer in the proguard cache format.
    pub fn write<W: Write>(mapping: &ProguardMapping, writer: &mut W) -> std::io::Result<()> {
        let mut string_table = StringTable::new();

        let parsed = ParsedProguardMapping::parse(*mapping, true);

        // Initialize class mappings with obfuscated -> original name data. The mappings will be filled in afterwards.
        let mut classes: BTreeMap<&str, ClassInProgress> = parsed
            .class_names
            .iter()
            .map(|(obfuscated, original)| {
                let obfuscated_name_offset = string_table.insert(obfuscated.as_str()) as u32;
                let original_name_offset = string_table.insert(original.as_str()) as u32;
                let is_synthesized = parsed
                    .class_infos
                    .get(original)
                    .map(|ci| ci.is_synthesized)
                    .unwrap_or_default();
                let class = ClassInProgress {
                    class: Class {
                        original_name_offset,
                        obfuscated_name_offset,
                        is_synthesized: is_synthesized as u8,
                        ..Default::default()
                    },
                    ..Default::default()
                };

                (obfuscated.as_str(), class)
            })
            .collect();

        for ((obfuscated_class, obfuscated_method), members) in &parsed.members {
            let current_class = classes.entry(obfuscated_class.as_str()).or_default();

            let obfuscated_method_offset = string_table.insert(obfuscated_method.as_str()) as u32;

            let method_mappings = current_class
                .members
                .entry(obfuscated_method.as_str())
                .or_default();

            for member in members.all.iter().copied() {
                method_mappings.push(Self::resolve_mapping(
                    &mut string_table,
                    &parsed,
                    obfuscated_method_offset,
                    member,
                ));
                current_class.class.members_len += 1;
            }

            for (args, param_members) in members.by_params.iter() {
                let param_mappings = current_class
                    .members_by_params
                    .entry((obfuscated_method.as_str(), args))
                    .or_default();

                for member in param_members {
                    param_mappings.push(Self::resolve_mapping(
                        &mut string_table,
                        &parsed,
                        obfuscated_method_offset,
                        *member,
                    ));
                    current_class.class.members_by_params_len += 1;
                }
            }
        }

        // At this point, we know how many members/members-by-params each class has because we kept count,
        // but we don't know where each class's entries start. We'll rectify that below.

        let mut writer = watto::Writer::new(writer);
        let string_bytes = string_table.into_bytes();

        let num_members = classes.values().map(|c| c.class.members_len).sum::<u32>();
        let num_members_by_params = classes
            .values()
            .map(|c| c.class.members_by_params_len)
            .sum::<u32>();

        let header = Header {
            magic: PRGCACHE_MAGIC,
            version: PRGCACHE_VERSION,
            num_classes: classes.len() as u32,
            num_members,
            num_members_by_params,
            string_bytes: string_bytes.len() as u32,
        };

        writer.write_all(header.as_bytes())?;
        writer.align_to(8)?;

        let mut members = Vec::new();
        let mut members_by_params = Vec::new();

        for mut c in classes.into_values() {
            // We can now set the class's members_offset/members_by_params_offset.
            c.class.members_offset = members.len() as u32;
            c.class.members_by_params_offset = members.len() as u32;
            members.extend(c.members.into_values().flat_map(|m| m.into_iter()));
            members_by_params.extend(
                c.members_by_params
                    .into_values()
                    .flat_map(|m| m.into_iter()),
            );
            writer.write_all(c.class.as_bytes())?;
        }
        writer.align_to(8)?;

        writer.write_all(members.as_bytes())?;
        writer.align_to(8)?;

        writer.write_all(members_by_params.as_bytes())?;
        writer.align_to(8)?;

        writer.write_all(&string_bytes)?;

        Ok(())
    }

    fn resolve_mapping(
        string_table: &mut StringTable,
        parsed: &ParsedProguardMapping<'_>,
        obfuscated_name_offset: u32,
        member: builder::Member,
    ) -> Member {
        let original_file = parsed
            .class_infos
            .get(&member.method.receiver.name())
            .and_then(|class| class.source_file);

        let original_file_offset =
            original_file.map_or(u32::MAX, |s| string_table.insert(s) as u32);
        let original_name_offset = string_table.insert(member.method.name.as_str()) as u32;

        // Only fill in `original_class` if it is _not_ the current class
        let original_class_offset = match member.method.receiver {
            builder::MethodReceiver::ThisClass(_) => u32::MAX,
            builder::MethodReceiver::OtherClass(name) => string_table.insert(name.as_str()) as u32,
        };

        let params_offset = string_table.insert(member.method.arguments) as u32;

        let method_info = parsed
            .method_infos
            .get(&member.method)
            .copied()
            .unwrap_or_default();
        let is_synthesized = method_info.is_synthesized as u8;

        Member {
            startline: member.startline as u32,
            endline: member.endline as u32,
            original_class_offset,
            original_file_offset,
            original_name_offset,
            original_startline: member.original_startline as u32,
            original_endline: member.original_endline.map_or(u32::MAX, |l| l as u32),
            obfuscated_name_offset,
            params_offset,
            is_synthesized,
            _reserved: [0; 3],
        }
    }

    /// Tests the integrity of this cache.
    ///
    /// Specifically it checks the following:
    /// * All string offsets in class and member entries are either `u32::MAX` or defined.
    /// * Member entries are ordered by the class they belong to.
    /// * All `is_synthesized` fields on classes and members are either `0` or `1`.
    pub fn test(&self) {
        let mut prev_end = 0;
        for class in self.classes {
            assert!(self.read_string(class.obfuscated_name_offset).is_ok());
            assert!(self.read_string(class.original_name_offset).is_ok());
            assert!(class.is_synthesized == 0 || class.is_synthesized == 1);

            if class.file_name_offset != u32::MAX {
                assert!(self.read_string(class.file_name_offset).is_ok());
            }

            assert_eq!(class.members_offset, prev_end);
            prev_end += class.members_len;
            assert!(prev_end as usize <= self.members.len());
            let Some(members) = self.get_class_members(class) else {
                continue;
            };

            for member in members {
                assert!(self.read_string(member.obfuscated_name_offset).is_ok());
                assert!(self.read_string(member.original_name_offset).is_ok());
                assert!(member.is_synthesized == 0 || member.is_synthesized == 1);

                if member.params_offset != u32::MAX {
                    assert!(self.read_string(member.params_offset).is_ok());
                }

                if member.original_class_offset != u32::MAX {
                    assert!(self.read_string(member.original_class_offset).is_ok());
                }

                if member.original_file_offset != u32::MAX {
                    assert!(self.read_string(member.original_file_offset).is_ok());
                }
            }
        }
    }

    pub(crate) fn read_string(&self, offset: u32) -> Result<&'data str, watto::ReadStringError> {
        StringTable::read(self.string_bytes, offset as usize)
    }
}

/// A class that is currently being constructed in the course of writing a [`ProguardCache`].
#[derive(Debug, Clone, Default)]
struct ClassInProgress<'data> {
    /// The class record.
    class: Class,
    /// The members records for the class, grouped by method name.
    members: BTreeMap<&'data str, Vec<Member>>,
    /// The member records for the class, grouped by method name and parameter string.
    members_by_params: BTreeMap<(&'data str, &'data str), Vec<Member>>,
}
