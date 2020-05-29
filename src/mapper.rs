use std::collections::{BTreeMap, HashMap};
use std::{fmt::Write, iter::FusedIterator};

use crate::mapping::MappingRecord;
use crate::stacktrace::StackFrame;

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
    members: BTreeMap<&'s str, Vec<MemberMapping<'s>>>,
}

type MemberIter<'m> = std::iter::Fuse<std::slice::Iter<'m, MemberMapping<'m>>>;

/// An Iterator over remapped StackFrames.
#[derive(Debug)]
pub struct RemappedFrameIter<'m> {
    inner: Option<(StackFrame<'m>, MemberIter<'m>)>,
}

impl<'m> RemappedFrameIter<'m> {
    fn empty() -> Self {
        Self { inner: None }
    }
    fn members(frame: StackFrame<'m>, members: MemberIter<'m>) -> Self {
        Self {
            inner: Some((frame, members)),
        }
    }
}

impl<'m> Iterator for RemappedFrameIter<'m> {
    type Item = StackFrame<'m>;
    fn next(&mut self) -> Option<Self::Item> {
        let (frame, ref mut members) = self.inner.as_mut()?;

        for member in members {
            // skip any members which do not match our the frames line
            if member.endline > 0 && (frame.line < member.startline || frame.line > member.endline)
            {
                continue;
            }
            // parents of inlined frames don’t have an `endline`, and
            // the top inlined frame need to be correctly offset.
            let line = if member.original_endline.is_none() {
                member.original_startline
            } else {
                member.original_startline + frame.line - member.startline
            };
            // when an inlined function is from a foreign class, we
            // don’t know the file it is defined in.
            let file = if member.original_class.is_some() {
                None
            } else {
                frame.file.clone()
            };
            let class = match member.original_class {
                Some(class) => class.into(),
                _ => frame.class.clone(),
            };
            return Some(StackFrame {
                class,
                method: member.original.into(),
                file,
                line,
            });
        }

        None
    }
}

impl FusedIterator for RemappedFrameIter<'_> {}

/// A Proguard Remapper.
///
/// This can remap frames one at a time, or the complete raw stacktrace.
#[derive(Debug)]
pub struct Mapper<'s> {
    classes: HashMap<&'s str, ClassMapping<'s>>,
}

impl<'s> Mapper<'s> {
    /// Create a new Proguard Remapper.
    pub fn new(mapping: &'s [u8]) -> Self {
        let mut classes = HashMap::new();
        let mut class = ClassMapping {
            original: "",
            obfuscated: "",
            members: BTreeMap::new(),
        };

        for record in mapping
            .split(|c| *c == b'\n' || *c == b'\r')
            .filter(|s| !s.is_empty())
            .filter_map(MappingRecord::try_parse)
        {
            match record {
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
                        members: BTreeMap::new(),
                    }
                }
                MappingRecord::Method {
                    original,
                    obfuscated,
                    original_class,
                    line_mapping,
                    ..
                } => {
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
                _ => {}
            }
        }
        if !class.original.is_empty() {
            classes.insert(class.obfuscated, class);
        }

        Self { classes }
    }

    /// Remaps an obfuscated Class.
    ///
    /// # Examples
    ///
    /// ```
    /// use proguard::Mapper;
    ///
    /// let mapping = br#"android.arch.core.executor.ArchTaskExecutor -> a.a.a.a.c:"#;
    /// let mapper = Mapper::new(mapping);
    ///
    /// let mapped = mapper.remap_class("a.a.a.a.c");
    /// assert_eq!(mapped, Some("android.arch.core.executor.ArchTaskExecutor"));
    /// ```
    pub fn remap_class(&'s self, class: &str) -> Option<&'s str> {
        self.classes.get(class).map(|class| class.original)
    }

    /// Remaps a single Stackframe.
    ///
    /// Returns zero or more [`StackFrame`]s, based on the information in
    /// the proguard mapping. This can return more than one frame in the case
    /// of inlined functions. In that case, frames are sorted top to bottom.
    ///
    /// [`StackFrame`]: struct.StackFrame.html
    pub fn remap_frame(&'s self, frame: &StackFrame<'s>) -> RemappedFrameIter<'s> {
        if let Some(class) = self.classes.get(frame.class.as_ref()) {
            if let Some(members) = class.members.get(frame.method.as_ref()) {
                let mut frame = frame.clone();
                frame.class = class.original.into();
                return RemappedFrameIter::members(frame, members.iter().fuse());
            }
        }
        RemappedFrameIter::empty()
    }

    /// Remaps a complete Java StackTrace.
    pub fn remap_stacktrace(&'s self, input: &'s str) -> Result<String, std::fmt::Error> {
        let mut stacktrace = String::new();
        for line in input.lines() {
            match StackFrame::try_parse(line.as_ref()) {
                None => writeln!(&mut stacktrace, "{}", line)?,
                Some(frame) => {
                    for line in self.remap_frame(&frame) {
                        writeln!(
                            &mut stacktrace,
                            "    at {}.{}({}:{})",
                            line.class,
                            line.method,
                            line.file.as_deref().unwrap_or("<unknown>"),
                            line.line
                        )?;
                    }
                }
            }
        }
        Ok(stacktrace)
    }
}
