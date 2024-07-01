use std::fmt;

use crate::ProguardCache;

use super::raw;

/// A variant of a class entry in a proguard cache file with
/// a nice-ish debug representation.
pub struct ClassDebug<'a, 'data> {
    pub(crate) cache: &'a ProguardCache<'data>,
    pub(crate) raw: raw::Class,
}

impl ClassDebug<'_, '_> {
    fn obfuscated_name(&self) -> &str {
        self.cache
            .read_string(self.raw.obfuscated_name_offset)
            .unwrap()
    }

    fn original_name(&self) -> &str {
        self.cache
            .read_string(self.raw.original_name_offset)
            .unwrap()
    }

    fn file_name(&self) -> Option<&str> {
        self.cache.read_string(self.raw.file_name_offset).ok()
    }
}

impl fmt::Debug for ClassDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Class")
            .field("obfuscated_name", &self.obfuscated_name())
            .field("original_name", &self.original_name())
            .field("file_name", &self.file_name())
            .finish()
    }
}

impl fmt::Display for ClassDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}:", self.original_name(), self.obfuscated_name())?;
        if let Some(file_name) = self.file_name() {
            writeln!(f)?;
            write!(f, r##"# {{"id":"sourceFile","fileName":"{file_name}"}}"##)?;
        }
        Ok(())
    }
}

/// A variant of a member entry in a proguard cache file with
/// a nice-ish debug representation.
pub struct MemberDebug<'a, 'data> {
    pub(crate) cache: &'a ProguardCache<'data>,
    pub(crate) raw: raw::Member,
}

impl MemberDebug<'_, '_> {
    fn original_class(&self) -> Option<&str> {
        self.cache.read_string(self.raw.original_class_offset).ok()
    }

    fn original_file(&self) -> Option<&str> {
        self.cache.read_string(self.raw.original_file_offset).ok()
    }

    fn params(&self) -> &str {
        self.cache
            .read_string(self.raw.params_offset)
            .unwrap_or_default()
    }

    fn obfuscated_name(&self) -> &str {
        self.cache
            .read_string(self.raw.obfuscated_name_offset)
            .unwrap()
    }

    fn original_name(&self) -> &str {
        self.cache
            .read_string(self.raw.original_name_offset)
            .unwrap()
    }

    fn original_endline(&self) -> Option<u32> {
        if self.raw.original_endline != u32::MAX {
            Some(self.raw.original_endline)
        } else {
            None
        }
    }
}

impl fmt::Debug for MemberDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Member")
            .field("obfuscated_name", &self.obfuscated_name())
            .field("startline", &self.raw.startline)
            .field("endline", &self.raw.endline)
            .field("original_name", &self.original_name())
            .field("original_class", &self.original_class())
            .field("original_file", &self.original_file())
            .field("original_startline", &self.raw.original_startline)
            .field("original_endline", &self.original_endline())
            .field("params", &self.params())
            .finish()
    }
}

impl fmt::Display for MemberDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // XXX: We could print the actual return type here if we saved it in the formot.
        // Wonder if it's worth it, since we'd only use it in this display impl.
        write!(f, "    {}:{}:<ret> ", self.raw.startline, self.raw.endline)?;
        if let Some(original_class) = self.original_class() {
            write!(f, "{original_class}.")?;
        }
        write!(
            f,
            "{}({}):{}",
            self.original_name(),
            self.params(),
            self.raw.original_startline
        )?;
        if let Some(end) = self.original_endline() {
            write!(f, ":{end}")?;
        }
        write!(f, " -> {}", self.obfuscated_name())?;
        Ok(())
    }
}

pub struct CacheDebug<'a, 'data> {
    cache: &'a ProguardCache<'data>,
}

impl fmt::Display for CacheDebug<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for class in self.cache.classes {
            writeln!(
                f,
                "{}",
                ClassDebug {
                    raw: class.clone(),
                    cache: self.cache
                }
            )?;
            let Some(members) = self.cache.get_class_members(class) else {
                continue;
            };

            for member in members {
                writeln!(
                    f,
                    "{}",
                    MemberDebug {
                        raw: member.clone(),
                        cache: self.cache
                    }
                )?;
            }
        }
        Ok(())
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

    /// Returns an iterator over by-params member entries in this cache file that can be debug printed.
    pub fn debug_members_by_params<'r>(&'r self) -> impl Iterator<Item = MemberDebug<'r, 'data>> {
        self.members_by_params.iter().map(move |m| MemberDebug {
            cache: self,
            raw: m.clone(),
        })
    }

    /// Creates a view of the cache that implements `Display`.
    ///
    /// The `Display` impl is very similar to the original proguard format.
    pub fn display(&self) -> CacheDebug {
        CacheDebug { cache: self }
    }
}
