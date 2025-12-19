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

/// The current version of the ProguardCache format.
pub const PRGCACHE_VERSION: u32 = 4;

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
    /// The total number of outline mapping pairs across all members.
    pub(crate) num_outline_pairs: u32,
    /// The total number of rewrite rule entries across all members.
    pub(crate) num_rewrite_rule_entries: u32,
    /// The total number of rewrite rule components across all members.
    pub(crate) num_rewrite_rule_components: u32,
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
    /// Offset into the outline pairs section for this member's outline callsite mapping.
    pub(crate) outline_pairs_offset: u32,
    /// Number of outline pairs for this member.
    pub(crate) outline_pairs_len: u32,
    /// Offset into the rewrite rule entries section for this member.
    pub(crate) rewrite_rules_offset: u32,
    /// Number of rewrite rule entries for this member.
    pub(crate) rewrite_rules_len: u32,
    /// Whether this member was synthesized by the compiler.
    ///
    /// `0` means `false`, all other values mean `true`.
    pub(crate) is_synthesized: u8,
    /// Whether this member refers to an outline method.
    ///
    /// `0` means `false`, all other values mean `true`.
    pub(crate) is_outline: u8,
    /// Reserved space.
    pub(crate) _reserved: [u8; 2],
}

impl Member {
    /// Returns true if this member was synthesized by the compiler.
    pub(crate) fn is_synthesized(&self) -> bool {
        self.is_synthesized != 0
    }
    /// Returns true if this member refers to an outline method.
    pub(crate) fn is_outline(&self) -> bool {
        self.is_outline != 0
    }
}

unsafe impl Pod for Header {}
unsafe impl Pod for Class {}
unsafe impl Pod for Member {}

/// A single outline mapping pair: outline position -> callsite line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct OutlinePair {
    pub(crate) outline_pos: u32,
    pub(crate) callsite_line: u32,
}

unsafe impl Pod for OutlinePair {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct RewriteRuleEntry {
    pub(crate) conditions_offset: u32,
    pub(crate) conditions_len: u32,
    pub(crate) actions_offset: u32,
    pub(crate) actions_len: u32,
}

unsafe impl Pod for RewriteRuleEntry {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct RewriteComponent {
    pub(crate) kind: u32,
    pub(crate) value: u32,
}

unsafe impl Pod for RewriteComponent {}

pub(crate) const REWRITE_CONDITION_THROWS: u32 = 0;
pub(crate) const REWRITE_CONDITION_UNKNOWN: u32 = u32::MAX;

pub(crate) const REWRITE_ACTION_REMOVE_INNER_FRAMES: u32 = 0;
pub(crate) const REWRITE_ACTION_UNKNOWN: u32 = u32::MAX;

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
    /// A flat list of outline mapping pairs.
    pub(crate) outline_pairs: &'data [OutlinePair],
    /// A flat list of rewrite rule entries.
    pub(crate) rewrite_rule_entries: &'data [RewriteRuleEntry],
    /// A flat list of rewrite rule components.
    pub(crate) rewrite_rule_components: &'data [RewriteComponent],
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

        let (_, rest) = watto::align_to(rest, 8).ok_or(CacheErrorKind::InvalidMembers)?;
        let (outline_pairs, rest) =
            OutlinePair::slice_from_prefix(rest, header.num_outline_pairs as usize)
                .ok_or(CacheErrorKind::InvalidMembers)?;

        let (_, rest) = watto::align_to(rest, 8).ok_or(CacheErrorKind::InvalidMembers)?;
        let (rewrite_rule_entries, rest) =
            RewriteRuleEntry::slice_from_prefix(rest, header.num_rewrite_rule_entries as usize)
                .ok_or(CacheErrorKind::InvalidMembers)?;

        let (_, rest) = watto::align_to(rest, 8).ok_or(CacheErrorKind::InvalidMembers)?;
        let (rewrite_rule_components, rest) =
            RewriteComponent::slice_from_prefix(rest, header.num_rewrite_rule_components as usize)
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
            outline_pairs,
            rewrite_rule_entries,
            rewrite_rule_components,
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
                let class_info = parsed.class_infos.get(original);
                let is_synthesized = class_info.map(|ci| ci.is_synthesized).unwrap_or_default();
                let file_name_offset = class_info
                    .and_then(|ci| ci.source_file)
                    .map_or(u32::MAX, |s| string_table.insert(s) as u32);
                let class = ClassInProgress {
                    class: Class {
                        original_name_offset,
                        obfuscated_name_offset,
                        file_name_offset,
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

            let has_line_info = members.all.iter().any(|m| m.endline > 0);
            for member in members.all.iter() {
                // Skip members without line information if there are members with line information
                if has_line_info && member.startline == 0 && member.endline == 0 {
                    continue;
                }
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

                for member in param_members.iter() {
                    param_mappings.push(Self::resolve_mapping(
                        &mut string_table,
                        &parsed,
                        obfuscated_method_offset,
                        member,
                    ));
                    current_class.class.members_by_params_len += 1;
                }
            }
        }

        // At this point, we know how many members/members-by-params each class has because we kept count,
        // but we don't know where each class's entries start. We'll rectify that below.

        let string_bytes = string_table.into_bytes();

        let num_members = classes.values().map(|c| c.class.members_len).sum::<u32>();
        let num_members_by_params = classes
            .values()
            .map(|c| c.class.members_by_params_len)
            .sum::<u32>();

        // Build output vectors first to know outline pair count.
        let mut out_classes: Vec<Class> = Vec::with_capacity(classes.len());
        let mut members: Vec<Member> = Vec::with_capacity(num_members as usize);
        let mut members_by_params: Vec<Member> = Vec::with_capacity(num_members_by_params as usize);
        let mut outline_pairs: Vec<OutlinePair> = Vec::new();
        let mut rewrite_rule_entries: Vec<RewriteRuleEntry> = Vec::new();
        let mut rewrite_rule_components: Vec<RewriteComponent> = Vec::new();

        for mut c in classes.into_values() {
            // Set offsets relative to current vector sizes
            c.class.members_offset = members.len() as u32;
            c.class.members_by_params_offset = members_by_params.len() as u32;

            // Serialize members without params
            for (_method, ms) in c.members {
                for mut mp in ms {
                    let start = outline_pairs.len() as u32;
                    if !mp.outline_pairs.is_empty() {
                        mp.member.outline_pairs_offset = start;
                        mp.member.outline_pairs_len = mp.outline_pairs.len() as u32;
                        outline_pairs.extend(mp.outline_pairs);
                    } else {
                        mp.member.outline_pairs_offset = start;
                        mp.member.outline_pairs_len = 0;
                    }

                    let rule_start = rewrite_rule_entries.len() as u32;
                    let mut rule_count = 0;
                    for rule in mp.rewrite_rules {
                        let cond_start = rewrite_rule_components.len() as u32;
                        rewrite_rule_components.extend(rule.conditions);
                        let cond_len = rewrite_rule_components.len() as u32 - cond_start;
                        let action_start = rewrite_rule_components.len() as u32;
                        rewrite_rule_components.extend(rule.actions);
                        let action_len = rewrite_rule_components.len() as u32 - action_start;
                        rewrite_rule_entries.push(RewriteRuleEntry {
                            conditions_offset: cond_start,
                            conditions_len: cond_len,
                            actions_offset: action_start,
                            actions_len: action_len,
                        });
                        rule_count += 1;
                    }
                    mp.member.rewrite_rules_offset = rule_start;
                    mp.member.rewrite_rules_len = rule_count;

                    members.push(mp.member);
                }
            }

            // Serialize members by params
            for (_key, ms) in c.members_by_params {
                for mut mp in ms {
                    let start = outline_pairs.len() as u32;
                    if !mp.outline_pairs.is_empty() {
                        mp.member.outline_pairs_offset = start;
                        mp.member.outline_pairs_len = mp.outline_pairs.len() as u32;
                        outline_pairs.extend(mp.outline_pairs);
                    } else {
                        mp.member.outline_pairs_offset = start;
                        mp.member.outline_pairs_len = 0;
                    }

                    let rule_start = rewrite_rule_entries.len() as u32;
                    let mut rule_count = 0;
                    for rule in mp.rewrite_rules {
                        let cond_start = rewrite_rule_components.len() as u32;
                        rewrite_rule_components.extend(rule.conditions);
                        let cond_len = rewrite_rule_components.len() as u32 - cond_start;
                        let action_start = rewrite_rule_components.len() as u32;
                        rewrite_rule_components.extend(rule.actions);
                        let action_len = rewrite_rule_components.len() as u32 - action_start;
                        rewrite_rule_entries.push(RewriteRuleEntry {
                            conditions_offset: cond_start,
                            conditions_len: cond_len,
                            actions_offset: action_start,
                            actions_len: action_len,
                        });
                        rule_count += 1;
                    }
                    mp.member.rewrite_rules_offset = rule_start;
                    mp.member.rewrite_rules_len = rule_count;

                    members_by_params.push(mp.member);
                }
            }

            out_classes.push(c.class);
        }

        let num_outline_pairs = outline_pairs.len() as u32;
        let num_rewrite_rule_entries = rewrite_rule_entries.len() as u32;
        let num_rewrite_rule_components = rewrite_rule_components.len() as u32;

        let header = Header {
            magic: PRGCACHE_MAGIC,
            version: PRGCACHE_VERSION,
            num_classes: out_classes.len() as u32,
            num_members,
            num_members_by_params,
            num_outline_pairs,
            num_rewrite_rule_entries,
            num_rewrite_rule_components,
            string_bytes: string_bytes.len() as u32,
        };

        let mut writer = watto::Writer::new(writer);
        writer.write_all(header.as_bytes())?;
        writer.align_to(8)?;

        // Write classes
        for c in out_classes.iter() {
            writer.write_all(c.as_bytes())?;
        }
        writer.align_to(8)?;

        // Write member sections
        writer.write_all(members.as_bytes())?;
        writer.align_to(8)?;

        writer.write_all(members_by_params.as_bytes())?;
        writer.align_to(8)?;

        // Write outline pairs
        writer.write_all(outline_pairs.as_bytes())?;
        writer.align_to(8)?;

        writer.write_all(rewrite_rule_entries.as_bytes())?;
        writer.align_to(8)?;

        writer.write_all(rewrite_rule_components.as_bytes())?;
        writer.align_to(8)?;

        // Write strings
        writer.write_all(&string_bytes)?;

        Ok(())
    }

    fn resolve_mapping(
        string_table: &mut StringTable,
        parsed: &ParsedProguardMapping<'_>,
        obfuscated_name_offset: u32,
        member: &builder::Member,
    ) -> MemberInProgress {
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
        let is_outline = method_info.is_outline as u8;

        let outline_pairs: Vec<OutlinePair> = member
            .outline_callsite_positions
            .as_ref()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| OutlinePair {
                        outline_pos: *k as u32,
                        callsite_line: *v as u32,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let rewrite_rules = member
            .rewrite_rules
            .iter()
            .map(|rule| {
                let mut conditions = Vec::new();
                for condition in &rule.conditions {
                    match condition {
                        builder::RewriteCondition::Throws(descriptor) => {
                            let offset = string_table.insert(descriptor) as u32;
                            conditions.push(RewriteComponent {
                                kind: REWRITE_CONDITION_THROWS,
                                value: offset,
                            });
                        }
                        builder::RewriteCondition::Unknown(value) => {
                            let offset = string_table.insert(value) as u32;
                            conditions.push(RewriteComponent {
                                kind: REWRITE_CONDITION_UNKNOWN,
                                value: offset,
                            });
                        }
                    }
                }

                let mut actions = Vec::new();
                for action in &rule.actions {
                    match action {
                        builder::RewriteAction::RemoveInnerFrames(count) => {
                            actions.push(RewriteComponent {
                                kind: REWRITE_ACTION_REMOVE_INNER_FRAMES,
                                value: *count as u32,
                            });
                        }
                        builder::RewriteAction::Unknown(value) => {
                            let offset = string_table.insert(value) as u32;
                            actions.push(RewriteComponent {
                                kind: REWRITE_ACTION_UNKNOWN,
                                value: offset,
                            });
                        }
                    }
                }

                RewriteRuleInProgress {
                    conditions,
                    actions,
                }
            })
            .collect();

        let member: Member = Member {
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
            is_outline,
            outline_pairs_offset: 0,
            outline_pairs_len: 0,
            rewrite_rules_offset: 0,
            rewrite_rules_len: 0,
            _reserved: [0; 2],
        };

        MemberInProgress {
            member,
            outline_pairs,
            rewrite_rules,
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
                assert!(member.is_outline == 0 || member.is_outline == 1);

                // Ensure outline pair range is within bounds
                let start = member.outline_pairs_offset as usize;
                let len = member.outline_pairs_len as usize;
                let end = start.saturating_add(len);
                assert!(end <= self.outline_pairs.len());

                let rule_start = member.rewrite_rules_offset as usize;
                let rule_len = member.rewrite_rules_len as usize;
                let rule_end = rule_start.saturating_add(rule_len);
                assert!(rule_end <= self.rewrite_rule_entries.len());
                for entry in &self.rewrite_rule_entries[rule_start..rule_end] {
                    let cond_start = entry.conditions_offset as usize;
                    let cond_len = entry.conditions_len as usize;
                    let cond_end = cond_start.saturating_add(cond_len);
                    assert!(cond_end <= self.rewrite_rule_components.len());

                    let action_start = entry.actions_offset as usize;
                    let action_len = entry.actions_len as usize;
                    let action_end = action_start.saturating_add(action_len);
                    assert!(action_end <= self.rewrite_rule_components.len());
                }

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
    members: BTreeMap<&'data str, Vec<MemberInProgress>>,
    /// The member records for the class, grouped by method name and parameter string.
    members_by_params: BTreeMap<(&'data str, &'data str), Vec<MemberInProgress>>,
}

#[derive(Debug, Clone, Default)]
struct MemberInProgress {
    member: Member,
    outline_pairs: Vec<OutlinePair>,
    rewrite_rules: Vec<RewriteRuleInProgress>,
}

#[derive(Debug, Clone, Default)]
struct RewriteRuleInProgress {
    conditions: Vec<RewriteComponent>,
    actions: Vec<RewriteComponent>,
}
