//! Contains functionality for parsing ProGuard mapping files into a
//! structured representation ([`ParsedProguardMapping`]) that can be
//! used to create a [`ProguardMapper`](crate::ProguardMapper) or
//! [`ProguardCache`](crate::ProguardCache).

use std::collections::{HashMap, HashSet};

use crate::{mapping::R8Header, ProguardMapping, ProguardRecord};

/// Newtype around &str for obfuscated class and method names.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct ObfuscatedName<'s>(&'s str);

impl<'s> ObfuscatedName<'s> {
    pub(crate) fn as_str(&self) -> &'s str {
        self.0
    }
}

impl std::ops::Deref for ObfuscatedName<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Newtype around &str for original class and method names.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct OriginalName<'s>(&'s str);

impl<'s> OriginalName<'s> {
    pub(crate) fn as_str(&self) -> &'s str {
        self.0
    }
}

impl std::ops::Deref for OriginalName<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Information about a class in a ProGuard file.
#[derive(Clone, Debug, Default)]
pub(crate) struct ClassInfo<'s> {
    /// The source file in which the class is defined.
    pub(crate) source_file: Option<&'s str>,
}

/// A key that uniquely identifies a method.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct MethodKey<'s> {
    /// The method's receiver.
    pub(crate) class: OriginalName<'s>,
    /// The method's name.
    pub(crate) name: OriginalName<'s>,
    /// The method's argument string.
    pub(crate) arguments: &'s str,
}

/// Information about a method in a ProGuard file.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MethodInfo {}

/// A member record in a Proguard file.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Member<'s> {
    /// The method the member refers to.
    pub(crate) method: MethodKey<'s>,
    /// The obfuscated/minified start line.
    pub(crate) startline: usize,
    /// The obfuscated/minified end line.
    pub(crate) endline: usize,
    /// The original start line.
    pub(crate) original_startline: usize,
    /// The original end line.
    pub(crate) original_endline: Option<usize>,
}

/// A collection of member records for a particular class
/// and obfuscated method.
#[derive(Clone, Debug, Default)]
pub(crate) struct Members<'s> {
    /// The complete list of members for the class and method.
    pub(crate) all: Vec<Member<'s>>,
    /// The complete list of members for the class and method,
    /// grouped by arguments string.
    pub(crate) by_params: HashMap<&'s str, Vec<Member<'s>>>,
}

/// A parsed representation of a [`ProguardMapping`].
#[derive(Clone, Debug, Default)]
pub(crate) struct ParsedProguardMapping<'s> {
    /// A mapping from obfuscated to original class names.
    pub(crate) class_names: HashMap<ObfuscatedName<'s>, OriginalName<'s>>,
    /// A mapping from original class names to class information.
    pub(crate) class_infos: HashMap<OriginalName<'s>, ClassInfo<'s>>,
    /// A mapping from method keys to method information.
    pub(crate) method_infos: HashMap<MethodKey<'s>, MethodInfo>,
    /// A mapping from obfuscated class and method names to members.
    pub(crate) members: HashMap<(ObfuscatedName<'s>, ObfuscatedName<'s>), Members<'s>>,
}

impl<'s> ParsedProguardMapping<'s> {
    pub(crate) fn parse(mapping: ProguardMapping<'s>, initialize_param_mapping: bool) -> Self {
        let mut slf = Self::default();
        let mut current_class_name = None;
        let mut current_class = ClassInfo::default();
        let mut unique_methods: HashSet<(&str, &str, &str)> = HashSet::new();

        let mut records = mapping.iter().filter_map(Result::ok).peekable();

        while let Some(record) = records.next() {
            match record {
                ProguardRecord::Field { .. } => {}
                ProguardRecord::Header { .. } => {}
                ProguardRecord::R8Header(_) => {
                    // R8 headers are already handled in the class case below.
                }
                ProguardRecord::Class {
                    original,
                    obfuscated,
                } => {
                    // Flush the previous class if there is one.
                    if let Some((obfuscated, original)) = current_class_name {
                        slf.class_names.insert(obfuscated, original);
                        slf.class_infos.insert(original, current_class);
                    }

                    current_class_name = Some((ObfuscatedName(obfuscated), OriginalName(original)));
                    current_class = ClassInfo::default();
                    unique_methods.clear();

                    // Consume R8 headers attached to this class.
                    while let Some(ProguardRecord::R8Header(r8_header)) = records.peek() {
                        match r8_header {
                            R8Header::SourceFile { file_name } => {
                                current_class.source_file = Some(file_name);
                            }
                            R8Header::Other => {}
                        }

                        records.next();
                    }
                }

                ProguardRecord::Method {
                    original,
                    obfuscated,
                    original_class,
                    line_mapping,
                    arguments,
                    ..
                } => {
                    let current_line = if initialize_param_mapping {
                        line_mapping
                    } else {
                        None
                    };
                    // in case the mapping has no line records, we use `0` here.
                    let (startline, endline) =
                        line_mapping.as_ref().map_or((0, 0), |line_mapping| {
                            (line_mapping.startline, line_mapping.endline)
                        });
                    let (original_startline, original_endline) =
                        line_mapping.map_or((0, None), |line_mapping| {
                            match line_mapping.original_startline {
                                Some(original_startline) => {
                                    (original_startline, line_mapping.original_endline)
                                }
                                None => (line_mapping.startline, Some(line_mapping.endline)),
                            }
                        });

                    let Some((current_class_obfuscated, current_class_original)) =
                        current_class_name
                    else {
                        // `current_class_name` is only `None` before the first class entry is encountered.
                        // If we hit this case, there's a member record before the first class record, which
                        // is an error. Properly handling this would be nice here, for now we return an empty `Self`.
                        return Self::default();
                    };

                    let members = slf
                        .members
                        .entry((current_class_obfuscated, ObfuscatedName(obfuscated)))
                        .or_default();

                    let method = MethodKey {
                        class: original_class
                            .map(OriginalName)
                            .unwrap_or(current_class_original),
                        name: OriginalName(original),
                        arguments,
                    };

                    // This does nothing for now because we are not saving any per-method information.
                    let _method_info: &mut MethodInfo = slf.method_infos.entry(method).or_default();

                    let member = Member {
                        method,
                        startline,
                        endline,
                        original_startline,
                        original_endline,
                    };

                    members.all.push(member);

                    if !initialize_param_mapping {
                        continue;
                    }
                    // If the next line has the same leading line range then this method
                    // has been inlined by the code minification process, as a result
                    // it can't show in method traces and can be safely ignored.
                    if let Some(ProguardRecord::Method {
                        line_mapping: Some(next_line),
                        ..
                    }) = records.peek()
                    {
                        if let Some(current_line_mapping) = current_line {
                            if (current_line_mapping.startline == next_line.startline)
                                && (current_line_mapping.endline == next_line.endline)
                            {
                                continue;
                            }
                        }
                    }

                    let key = (obfuscated, arguments, original);
                    if unique_methods.insert(key) {
                        members
                            .by_params
                            .entry(arguments)
                            .or_insert_with(|| Vec::with_capacity(1))
                            .push(member);
                    }
                } // end ProguardRecord::Method
            }
        }

        // Flush the last class
        if let Some((obfuscated, original)) = current_class_name {
            slf.class_names.insert(obfuscated, original);
            slf.class_infos.insert(original, current_class);
        }

        slf
    }
}
