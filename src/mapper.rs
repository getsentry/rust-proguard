use std::collections::HashMap;
use std::{borrow::Cow, fmt::Write};

use crate::mapping::{parse_mapping_line, MappingRecord};
use crate::stacktrace::{parse_stacktrace_line, StackFrame};

#[derive(Debug)]
struct MemberMapping<'s> {
    startline: usize,
    endline: usize,
    original_class: Option<&'s str>,
    original: &'s str,
    original_startline: usize,
    original_endline: Option<usize>,
}

#[derive(Debug)]
struct ClassMapping<'s> {
    original: &'s str,
    obfuscated: &'s str,
    members: HashMap<&'s str, Vec<MemberMapping<'s>>>,
}

/// A Proguard Remapper.
///
/// This can remap frames one at a time, or the complete raw stacktrace.
#[derive(Debug)]
pub struct Mapper<'s> {
    classes: HashMap<&'s str, ClassMapping<'s>>,
}

impl<'s> Mapper<'s> {
    /// Create a new Proguard Remapper.
    pub fn new(mapping: &'s str) -> Self {
        let mut classes = HashMap::new();
        let mut class = ClassMapping {
            original: "",
            obfuscated: "",
            members: HashMap::new(),
        };

        for record in mapping.lines().filter_map(parse_mapping_line) {
            match record {
                MappingRecord::Header { .. } => {}
                MappingRecord::Class {
                    original,
                    obfuscated,
                } => {
                    if !class.original.is_empty() {
                        classes.insert(class.obfuscated, class);
                    }
                    class = ClassMapping {
                        original,
                        obfuscated,
                        members: HashMap::new(),
                    }
                }
                MappingRecord::Member {
                    original,
                    obfuscated,
                    original_class,
                    line_mapping,
                    ..
                } => {
                    let (startline, endline, original_startline, original_endline) = line_mapping
                        .map_or((0, 0, 0, None), |line_mapping| {
                            (
                                line_mapping.startline,
                                line_mapping.endline,
                                line_mapping.original_startline.unwrap_or(0),
                                line_mapping.original_endline,
                            )
                        });
                    let members = class.members.entry(obfuscated).or_insert_with(|| vec![]);
                    members.push(MemberMapping {
                        startline,
                        endline,
                        original_class,
                        original,
                        original_startline,
                        original_endline,
                    });
                }
            }
        }
        if !class.original.is_empty() {
            classes.insert(class.obfuscated, class);
        }

        Self { classes }
    }

    /// Remaps a single Stackframe.
    ///
    /// Returns one or more [`StackFrame`]s, based on the information in
    /// the proguard mapping. This can return more than one frame in the case
    /// of inlined functions. In that case, frames are sorted top to bottom.
    ///
    /// [`StackFrame`]: struct.StackFrame.html
    pub fn remap_frame(
        &'s self,
        frame: &StackFrame<'s>,
    ) -> impl Iterator<Item = StackFrame<'s>> + 's {
        let mut frame = frame.clone();
        if let Some(class) = self.classes.get(frame.class.as_ref()) {
            frame.class = class.original.into();
            if let Some(members) = class.members.get(frame.method.as_ref()) {
                let mut mapped = vec![];
                for member in members {
                    // in case we don’t find a record based on the line number,
                    // we can still re-map the method name.
                    if member.original_class.is_none() {
                        frame.method = member.original.into();
                    }
                    // skip any members which do not match our the frames line
                    if frame.line < member.startline || frame.line > member.endline {
                        continue;
                    }
                    // parents of inlined frames don’t have an `endline`, and
                    // the top inlined frame need to be correctly offset.
                    let line = if member.original_endline.is_none() {
                        member.original_startline
                    } else {
                        member.original_startline + frame.line - member.startline
                    };
                    let file = member
                        .original_class
                        .map(|c| {
                            let c = c.rsplit('.').next().unwrap();
                            Cow::Owned(format!("{}.java", c))
                        })
                        .unwrap_or_else(|| frame.file.clone());
                    mapped.push(StackFrame {
                        class: member.original_class.unwrap_or(class.original).into(),
                        method: member.original.into(),
                        file,
                        line,
                    })
                }
                if !mapped.is_empty() {
                    return mapped.into_iter();
                }
            }
        }
        vec![frame].into_iter()
    }

    /// Remaps a complete Java StackTrace.
    pub fn remap_stacktrace(&'s self, input: &'s str) -> Result<String, std::fmt::Error> {
        let mut stacktrace = String::new();
        for line in input.lines() {
            match parse_stacktrace_line(line) {
                None => writeln!(&mut stacktrace, "{}", line)?,
                Some(frame) => {
                    for line in self.remap_frame(&frame) {
                        writeln!(
                            &mut stacktrace,
                            "    at {}.{}({}:{})",
                            line.class, line.method, line.file, line.line
                        )?;
                    }
                }
            }
        }
        Ok(stacktrace)
    }
}
