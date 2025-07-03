use std::collections::{HashMap, HashSet};

use crate::{mapping::R8Header, ProguardMapping, ProguardRecord};

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

#[derive(Clone, Debug, Default)]
pub(crate) struct ClassInfo<'s> {
    pub(crate) source_file: Option<&'s str>,
    pub(crate) members: HashMap<ObfuscatedName<'s>, Members<'s>>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct MethodKey<'s> {
    pub(crate) class: OriginalName<'s>,
    pub(crate) name: OriginalName<'s>,
    pub(crate) arguments: &'s str,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MethodInfo {}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Member<'s> {
    pub(crate) method: MethodKey<'s>,
    pub(crate) startline: usize,
    pub(crate) endline: usize,
    pub(crate) original_startline: usize,
    pub(crate) original_endline: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Members<'s> {
    pub(crate) all: Vec<Member<'s>>,
    pub(crate) by_params: HashMap<&'s str, Vec<Member<'s>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedProguardMapping<'s> {
    pub(crate) class_names: HashMap<ObfuscatedName<'s>, OriginalName<'s>>,
    pub(crate) classes: HashMap<OriginalName<'s>, ClassInfo<'s>>,
}

impl<'s> ParsedProguardMapping<'s> {
    pub(crate) fn parse(mapping: ProguardMapping<'s>, initialize_param_mapping: bool) -> Self {
        let mut classes = HashMap::new();
        let mut methods = HashMap::new();
        let mut class_names = HashMap::new();
        // let mut method_name_mapping: HashMap::new();
        let mut current_class_name = None;
        let mut current_class = ClassInfo::default();
        let mut unique_methods: HashSet<(&str, &str, &str)> = HashSet::new();

        let mut records = mapping.iter().filter_map(Result::ok).peekable();

        while let Some(record) = records.next() {
            match record {
                ProguardRecord::Field { .. } => {}
                ProguardRecord::Header { .. } => {}
                ProguardRecord::R8Header(_) => {}
                ProguardRecord::Class {
                    original,
                    obfuscated,
                } => {
                    // Flush the previous class if there is one.
                    if let Some(name) = current_class_name {
                        classes.insert(name, current_class);
                    }
                    let key = OriginalName(original);
                    current_class_name = Some(key);
                    current_class = ClassInfo::default();
                    class_names.insert(ObfuscatedName(obfuscated), key);
                    unique_methods.clear();

                    // consume R8 headers attached to this class
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

                    let members = current_class
                        .members
                        .entry(ObfuscatedName(obfuscated))
                        .or_default();

                    let method = MethodKey {
                        class: original_class
                            .map(OriginalName)
                            .or(current_class_name)
                            .unwrap(),
                        name: OriginalName(original),
                        arguments,
                    };

                    let _method_info: &mut MethodInfo = methods.entry(method).or_default();
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

        if let Some(name) = current_class_name {
            classes.insert(name, current_class);
        }

        Self {
            classes,
            class_names,
        }
    }
}
