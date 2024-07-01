use std::fmt;

use watto::StringTable;

use crate::ProguardCache;

use super::raw;

/// A variant of a class entry in a proguard cache file with
/// a nice-ish debug representation.
pub struct ClassDebug<'a, 'data> {
    pub(crate) cache: &'a ProguardCache<'data>,
    pub(crate) raw: raw::Class,
}

impl fmt::Debug for ClassDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Class")
            .field(
                "obfuscated_name",
                &StringTable::read(
                    self.cache.string_bytes,
                    self.raw.obfuscated_name_offset as usize,
                ),
            )
            .field(
                "original_name",
                &StringTable::read(
                    self.cache.string_bytes,
                    self.raw.original_name_offset as usize,
                ),
            )
            .field(
                "file_name",
                &StringTable::read(self.cache.string_bytes, self.raw.file_name_offset as usize),
            )
            .finish()
    }
}

/// A variant of a member entry in a proguard cache file with
/// a nice-ish debug representation.
pub struct MemberDebug<'a, 'data> {
    pub(crate) cache: &'a ProguardCache<'data>,
    pub(crate) raw: raw::Member,
}

impl fmt::Debug for MemberDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Member")
            .field(
                "obfuscated_name",
                &StringTable::read(
                    self.cache.string_bytes,
                    self.raw.obfuscated_name_offset as usize,
                )
                .unwrap(),
            )
            .field("startline", &self.raw.startline)
            .field("endline", &self.raw.endline)
            .field(
                "original_class",
                &StringTable::read(
                    self.cache.string_bytes,
                    self.raw.original_class_offset as usize,
                ),
            )
            .field(
                "original_file",
                &StringTable::read(
                    self.cache.string_bytes,
                    self.raw.original_file_offset as usize,
                ),
            )
            .field(
                "original_name",
                &StringTable::read(
                    self.cache.string_bytes,
                    self.raw.original_name_offset as usize,
                ),
            )
            .field("original_startline", &self.raw.original_startline)
            .field("original_endline", &self.raw.original_endline)
            .field(
                "params",
                &StringTable::read(self.cache.string_bytes, self.raw.params_offset as usize),
            )
            .finish()
    }
}

impl<'data> ProguardCache<'data> {
    /// Returns an iterator over class entries in this cache file that can be debug printed.
    pub fn debug_classes<'r>(&'r self) -> impl Iterator<Item = ClassDebug<'r, 'data>> {
        self.classes.iter().map(move |c| ClassDebug {
            cache: self,
            raw: c.clone(),
        })
    }

    /// Returns an iterator over member entries in this cache file that can be debug printed.
    pub fn debug_members<'r>(&'r self) -> impl Iterator<Item = MemberDebug<'r, 'data>> {
        self.members.iter().map(move |m| MemberDebug {
            cache: self,
            raw: m.clone(),
        })
    }
}
