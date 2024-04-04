use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::{Error as FmtError, Write};
use std::iter::FusedIterator;

use crate::mapping::{ProguardMapping, ProguardRecord};
use crate::stacktrace::{self, StackFrame, StackTrace, Throwable};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MemberMapping<'s> {
    startline: usize,
    endline: usize,
    original_class: Option<&'s str>,
    original_file: Option<&'s str>,
    original: &'s str,
    original_startline: usize,
    original_endline: Option<usize>,
}

#[derive(Clone, Debug)]
struct ClassMembers<'s> {
    all_mappings: Vec<MemberMapping<'s>>,
    // method_params -> Vec[MemberMapping]
    mappings_by_params: HashMap<&'s str, Vec<MemberMapping<'s>>>,
}

#[derive(Clone, Debug)]
struct ClassMapping<'s> {
    original: &'s str,
    obfuscated: &'s str,
    file_name: Option<&'s str>,
    members: HashMap<&'s str, ClassMembers<'s>>,
}

type MemberIter<'m> = std::slice::Iter<'m, MemberMapping<'m>>;

/// An Iterator over remapped StackFrames.
#[derive(Clone, Debug, Default)]
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
        if frame.parameters.is_none() {
            iterate_with_lines(frame, members)
        } else {
            iterate_without_lines(frame, members)
        }
    }
}

fn extract_class_name(full_path: &str) -> Option<&str> {
    let after_last_period = full_path.split('.').last()?;
    // If the class is an inner class, we need to extract the outer class name
    after_last_period.split('$').next()
}

fn iterate_with_lines<'a>(
    frame: &mut StackFrame<'a>,
    members: &mut core::slice::Iter<'_, MemberMapping<'a>>,
) -> Option<StackFrame<'a>> {
    for member in members {
        // skip any members which do not match our frames line
        if member.endline > 0 && (frame.line < member.startline || frame.line > member.endline) {
            continue;
        }
        // parents of inlined frames don’t have an `endline`, and
        // the top inlined frame need to be correctly offset.
        let line = if member.original_endline.is_none()
            || member.original_endline == Some(member.original_startline)
        {
            member.original_startline
        } else {
            member.original_startline + frame.line - member.startline
        };
        let file = if let Some(file_name) = member.original_file {
            if file_name == "R8$$SyntheticClass" {
                extract_class_name(member.original_class.unwrap_or(frame.class))
            } else {
                member.original_file
            }
        } else if member.original_class.is_some() {
            // when an inlined function is from a foreign class, we
            // don’t know the file it is defined in.
            None
        } else {
            frame.file
        };
        let class = match member.original_class {
            Some(class) => class,
            _ => frame.class,
        };
        return Some(StackFrame {
            class,
            method: member.original,
            file,
            line,
            parameters: frame.parameters,
        });
    }
    None
}

fn iterate_without_lines<'a>(
    frame: &mut StackFrame<'a>,
    members: &mut core::slice::Iter<'_, MemberMapping<'a>>,
) -> Option<StackFrame<'a>> {
    let member = members.next()?;

    let class = match member.original_class {
        Some(class) => class,
        _ => frame.class,
    };
    Some(StackFrame {
        class,
        method: member.original,
        file: None,
        line: 0,
        parameters: frame.parameters,
    })
}

impl FusedIterator for RemappedFrameIter<'_> {}

/// A Proguard Remapper.
///
/// This can remap class names, stack frames one at a time, or the complete
/// raw stacktrace.
#[derive(Clone, Debug)]
pub struct ProguardMapper<'s> {
    classes: HashMap<&'s str, ClassMapping<'s>>,
}

impl<'s> From<&'s str> for ProguardMapper<'s> {
    fn from(s: &'s str) -> Self {
        let mapping = ProguardMapping::new(s.as_ref());
        Self::new(mapping)
    }
}

impl<'s> From<(&'s str, bool)> for ProguardMapper<'s> {
    fn from(t: (&'s str, bool)) -> Self {
        let mapping = ProguardMapping::new(t.0.as_ref());
        Self::new_with_param_mapping(mapping, t.1)
    }
}

impl<'s> ProguardMapper<'s> {
    /// Create a new ProguardMapper.
    pub fn new(mapping: ProguardMapping<'s>) -> Self {
        Self::create_proguard_mapper(mapping, false)
    }

    /// Create a new ProguardMapper with the extra mappings_by_params.
    /// This is useful when we want to deobfuscate frames with missing
    /// line information
    pub fn new_with_param_mapping(
        mapping: ProguardMapping<'s>,
        initialize_param_mapping: bool,
    ) -> Self {
        Self::create_proguard_mapper(mapping, initialize_param_mapping)
    }

    fn create_proguard_mapper(
        mapping: ProguardMapping<'s>,
        initialize_param_mapping: bool,
    ) -> Self {
        let mut classes = HashMap::new();
        let mut class = ClassMapping {
            original: "",
            obfuscated: "",
            file_name: None,
            members: HashMap::new(),
        };
        let mut unique_methods: HashSet<(&str, &str, &str)> = HashSet::new();

        let mut records = mapping.iter().filter_map(Result::ok).peekable();
        while let Some(record) = records.next() {
            match record {
                ProguardRecord::Header { key, value } => {
                    if key == "sourceFile" {
                        class.file_name = value;
                    }
                }
                ProguardRecord::Class {
                    original,
                    obfuscated,
                } => {
                    if !class.original.is_empty() {
                        classes.insert(class.obfuscated, class);
                    }
                    class = ClassMapping {
                        original,
                        obfuscated,
                        file_name: None,
                        members: HashMap::new(),
                    };
                    unique_methods.clear();
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
                        line_mapping.clone()
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

                    let members = class
                        .members
                        .entry(obfuscated)
                        .or_insert_with(|| ClassMembers {
                            all_mappings: Vec::with_capacity(1),
                            mappings_by_params: Default::default(),
                        });

                    let member_mapping = MemberMapping {
                        startline,
                        endline,
                        original_class,
                        original_file: class.file_name,
                        original,
                        original_startline,
                        original_endline,
                    };
                    members.all_mappings.push(member_mapping.clone());

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
                            .mappings_by_params
                            .entry(arguments)
                            .or_insert_with(|| Vec::with_capacity(1))
                            .push(member_mapping);
                    }
                } // end ProguardRecord::Method
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
    /// This works on the fully-qualified name of the class, with its complete
    /// module prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// let mapping = r#"android.arch.core.executor.ArchTaskExecutor -> a.a.a.a.c:"#;
    /// let mapper = proguard::ProguardMapper::from(mapping);
    ///
    /// let mapped = mapper.remap_class("a.a.a.a.c");
    /// assert_eq!(mapped, Some("android.arch.core.executor.ArchTaskExecutor"));
    /// ```
    pub fn remap_class(&'s self, class: &str) -> Option<&'s str> {
        self.classes.get(class).map(|class| class.original)
    }

    /// Remaps an obfuscated Class Method.
    ///
    /// The `class` argument has to be the fully-qualified obfuscated name of the
    /// class, with its complete module prefix.
    ///
    /// If the `method` can be resolved unambiguously, it will be returned
    /// alongside the remapped `class`, otherwise `None` is being returned.
    pub fn remap_method(&'s self, class: &str, method: &str) -> Option<(&'s str, &'s str)> {
        let class = self.classes.get(class)?;
        let mut members = class.members.get(method)?.all_mappings.iter();
        let first = members.next()?;

        // We conservatively check that all the mappings point to the same method,
        // as we don’t have line numbers to disambiguate.
        // We could potentially skip inlined functions here, but lets rather be conservative.
        let all_matching = members.all(|member| member.original == first.original);

        all_matching.then_some((class.original, first.original))
    }

    /// Remaps a single Stackframe.
    ///
    /// Returns zero or more [`StackFrame`]s, based on the information in
    /// the proguard mapping. This can return more than one frame in the case
    /// of inlined functions. In that case, frames are sorted top to bottom.
    pub fn remap_frame(&'s self, frame: &StackFrame<'s>) -> RemappedFrameIter<'s> {
        let Some(class) = self.classes.get(frame.class) else {
            return RemappedFrameIter::empty();
        };
        let Some(members) = class.members.get(frame.method) else {
            return RemappedFrameIter::empty();
        };

        let mut frame = frame.clone();
        frame.class = class.original;

        let mappings = if let Some(parameters) = frame.parameters {
            if let Some(typed_members) = members.mappings_by_params.get(parameters) {
                typed_members.iter()
            } else {
                return RemappedFrameIter::empty();
            }
        } else {
            members.all_mappings.iter()
        };

        return RemappedFrameIter::members(frame, mappings);
    }

    /// Remaps a throwable which is the first line of a full stacktrace.
    ///
    /// # Example
    ///
    /// ```
    /// use proguard::{ProguardMapper, Throwable};
    ///
    /// let mapping = "com.example.Mapper -> a.b:";
    /// let mapper = ProguardMapper::from(mapping);
    ///
    /// let throwable = Throwable::try_parse(b"a.b: Crash").unwrap();
    /// let mapped = mapper.remap_throwable(&throwable);
    ///
    /// assert_eq!(
    ///     Some(Throwable::with_message("com.example.Mapper", "Crash")),
    ///     mapped
    /// );
    /// ```
    pub fn remap_throwable<'a>(&'a self, throwable: &Throwable<'a>) -> Option<Throwable<'a>> {
        self.remap_class(throwable.class).map(|class| Throwable {
            class,
            message: throwable.message,
        })
    }

    /// Remaps a complete Java StackTrace, similar to [`Self::remap_stacktrace_typed`] but instead works on
    /// strings as input and output.
    pub fn remap_stacktrace(&self, input: &str) -> Result<String, std::fmt::Error> {
        let mut stacktrace = String::new();
        let mut lines = input.lines();

        if let Some(line) = lines.next() {
            match stacktrace::parse_throwable(line) {
                None => match stacktrace::parse_frame(line) {
                    None => writeln!(&mut stacktrace, "{}", line)?,
                    Some(frame) => format_frames(&mut stacktrace, line, self.remap_frame(&frame))?,
                },
                Some(throwable) => {
                    format_throwable(&mut stacktrace, line, self.remap_throwable(&throwable))?
                }
            }
        }

        for line in lines {
            match stacktrace::parse_frame(line) {
                None => match line
                    .strip_prefix("Caused by: ")
                    .and_then(stacktrace::parse_throwable)
                {
                    None => writeln!(&mut stacktrace, "{}", line)?,
                    Some(cause) => {
                        format_cause(&mut stacktrace, line, self.remap_throwable(&cause))?
                    }
                },
                Some(frame) => format_frames(&mut stacktrace, line, self.remap_frame(&frame))?,
            }
        }
        Ok(stacktrace)
    }

    /// Remaps a complete Java StackTrace.
    pub fn remap_stacktrace_typed<'a>(&'a self, trace: &StackTrace<'a>) -> StackTrace<'a> {
        let exception = trace
            .exception
            .as_ref()
            .and_then(|t| self.remap_throwable(t));

        let frames =
            trace
                .frames
                .iter()
                .fold(Vec::with_capacity(trace.frames.len()), |mut frames, f| {
                    let mut peek_frames = self.remap_frame(f).peekable();
                    if peek_frames.peek().is_some() {
                        frames.extend(peek_frames);
                    } else {
                        frames.push(f.clone());
                    }

                    frames
                });

        let cause = trace
            .cause
            .as_ref()
            .map(|c| Box::new(self.remap_stacktrace_typed(c)));

        StackTrace {
            exception,
            frames,
            cause,
        }
    }
}

fn format_throwable(
    stacktrace: &mut impl Write,
    line: &str,
    throwable: Option<Throwable<'_>>,
) -> Result<(), FmtError> {
    if let Some(throwable) = throwable {
        writeln!(stacktrace, "{}", throwable)
    } else {
        writeln!(stacktrace, "{}", line)
    }
}

fn format_frames<'s>(
    stacktrace: &mut impl Write,
    line: &str,
    remapped: impl Iterator<Item = StackFrame<'s>>,
) -> Result<(), FmtError> {
    let mut remapped = remapped.peekable();

    if remapped.peek().is_none() {
        return writeln!(stacktrace, "{}", line);
    }
    for line in remapped {
        writeln!(stacktrace, "    {}", line)?;
    }

    Ok(())
}

fn format_cause(
    stacktrace: &mut impl Write,
    line: &str,
    cause: Option<Throwable<'_>>,
) -> Result<(), FmtError> {
    if let Some(cause) = cause {
        writeln!(stacktrace, "Caused by: {}", cause)
    } else {
        writeln!(stacktrace, "{}", line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacktrace() {
        let mapping = "\
com.example.MainFragment$EngineFailureException -> com.example.MainFragment$d:
com.example.MainFragment$RocketException -> com.example.MainFragment$e:
com.example.MainFragment$onActivityCreated$4 -> com.example.MainFragment$g:
    1:1:void com.example.MainFragment$Rocket.startEngines():90:90 -> onClick
    1:1:void com.example.MainFragment$Rocket.fly():83 -> onClick
    1:1:void onClick(android.view.View):65 -> onClick
    2:2:void com.example.MainFragment$Rocket.fly():85:85 -> onClick
    2:2:void onClick(android.view.View):65 -> onClick
    ";
        let stacktrace = StackTrace {
            exception: Some(Throwable {
                class: "com.example.MainFragment$e",
                message: Some("Crash!"),
            }),
            frames: vec![
                StackFrame {
                    class: "com.example.MainFragment$g",
                    method: "onClick",
                    line: 2,
                    file: Some("SourceFile"),
                    parameters: None,
                },
                StackFrame {
                    class: "android.view.View",
                    method: "performClick",
                    line: 7393,
                    file: Some("View.java"),
                    parameters: None,
                },
            ],
            cause: Some(Box::new(StackTrace {
                exception: Some(Throwable {
                    class: "com.example.MainFragment$d",
                    message: Some("Engines overheating"),
                }),
                frames: vec![StackFrame {
                    class: "com.example.MainFragment$g",
                    method: "onClick",
                    line: 1,
                    file: Some("SourceFile"),
                    parameters: None,
                }],
                cause: None,
            })),
        };
        let expect = "\
com.example.MainFragment$RocketException: Crash!
    at com.example.MainFragment$Rocket.fly(<unknown>:85)
    at com.example.MainFragment$onActivityCreated$4.onClick(SourceFile:65)
    at android.view.View.performClick(View.java:7393)
Caused by: com.example.MainFragment$EngineFailureException: Engines overheating
    at com.example.MainFragment$Rocket.startEngines(<unknown>:90)
    at com.example.MainFragment$Rocket.fly(<unknown>:83)
    at com.example.MainFragment$onActivityCreated$4.onClick(SourceFile:65)\n";

        let mapper = ProguardMapper::from(mapping);

        assert_eq!(
            expect,
            mapper.remap_stacktrace_typed(&stacktrace).to_string()
        );
    }

    #[test]
    fn stacktrace_str() {
        let mapping = "\
com.example.MainFragment$EngineFailureException -> com.example.MainFragment$d:
com.example.MainFragment$RocketException -> com.example.MainFragment$e:
com.example.MainFragment$onActivityCreated$4 -> com.example.MainFragment$g:
    1:1:void com.example.MainFragment$Rocket.startEngines():90:90 -> onClick
    1:1:void com.example.MainFragment$Rocket.fly():83 -> onClick
    1:1:void onClick(android.view.View):65 -> onClick
    2:2:void com.example.MainFragment$Rocket.fly():85:85 -> onClick
    2:2:void onClick(android.view.View):65 -> onClick
    ";
        let stacktrace = "\
com.example.MainFragment$e: Crash!
    at com.example.MainFragment$g.onClick(SourceFile:2)
    at android.view.View.performClick(View.java:7393)
Caused by: com.example.MainFragment$d: Engines overheating
    at com.example.MainFragment$g.onClick(SourceFile:1)
    ... 13 more";
        let expect = "\
com.example.MainFragment$RocketException: Crash!
    at com.example.MainFragment$Rocket.fly(<unknown>:85)
    at com.example.MainFragment$onActivityCreated$4.onClick(SourceFile:65)
    at android.view.View.performClick(View.java:7393)
Caused by: com.example.MainFragment$EngineFailureException: Engines overheating
    at com.example.MainFragment$Rocket.startEngines(<unknown>:90)
    at com.example.MainFragment$Rocket.fly(<unknown>:83)
    at com.example.MainFragment$onActivityCreated$4.onClick(SourceFile:65)
    ... 13 more\n";

        let mapper = ProguardMapper::from(mapping);

        assert_eq!(expect, mapper.remap_stacktrace(stacktrace).unwrap());
    }
}
