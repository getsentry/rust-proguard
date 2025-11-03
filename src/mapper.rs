use std::collections::HashMap;
use std::fmt;
use std::fmt::{Error as FmtError, Write};
use std::iter::FusedIterator;

use crate::builder::{Member, MethodReceiver, ParsedProguardMapping};
use crate::java;
use crate::mapping::ProguardMapping;
use crate::stacktrace::{self, StackFrame, StackTrace, Throwable};

/// A deobfuscated method signature.
pub struct DeobfuscatedSignature {
    parameters: Vec<String>,
    return_type: String,
}

impl DeobfuscatedSignature {
    pub(crate) fn new(signature: (Vec<String>, String)) -> DeobfuscatedSignature {
        DeobfuscatedSignature {
            parameters: signature.0,
            return_type: signature.1,
        }
    }

    /// Returns the java return type of the method signature
    pub fn return_type(&self) -> &str {
        self.return_type.as_str()
    }

    /// Returns the list of paramater types of the method signature
    pub fn parameters_types(&self) -> impl Iterator<Item = &str> {
        self.parameters.iter().map(|s| s.as_ref())
    }

    /// formats types (param_type list, return_type) into a human-readable signature
    pub fn format_signature(&self) -> String {
        let mut signature = format!("({})", self.parameters.join(", "));
        if !self.return_type().is_empty() && self.return_type() != "void" {
            signature.push_str(": ");
            signature.push_str(self.return_type());
        }

        signature
    }
}

impl fmt::Display for DeobfuscatedSignature {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.format_signature())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MemberMapping<'s> {
    startline: usize,
    endline: usize,
    original_class: Option<&'s str>,
    original_file: Option<&'s str>,
    original: &'s str,
    original_startline: usize,
    original_endline: Option<usize>,
    is_synthesized: bool,
    is_outline: bool,
    outline_callsite_positions: Option<HashMap<usize, usize>>,
}

#[derive(Clone, Debug, Default)]
struct ClassMembers<'s> {
    all_mappings: Vec<MemberMapping<'s>>,
    // method_params -> Vec[MemberMapping]
    mappings_by_params: HashMap<&'s str, Vec<MemberMapping<'s>>>,
}

#[derive(Clone, Debug, Default)]
struct ClassMapping<'s> {
    original: &'s str,
    members: HashMap<&'s str, ClassMembers<'s>>,
    #[expect(
        unused,
        reason = "It is currently unknown what effect a synthesized class has."
    )]
    is_synthesized: bool,
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
    let after_last_period = full_path.split('.').next_back()?;
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
            method_synthesized: member.is_synthesized,
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
        method_synthesized: member.is_synthesized,
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
        let parsed = ParsedProguardMapping::parse(mapping, initialize_param_mapping);

        // Initialize class mappings with obfuscated -> original name data. The mappings will be filled in afterwards.
        let mut class_mappings: HashMap<&str, ClassMapping<'s>> = parsed
            .class_names
            .iter()
            .map(|(obfuscated, original)| {
                let is_synthesized = parsed
                    .class_infos
                    .get(original)
                    .map(|ci| ci.is_synthesized)
                    .unwrap_or_default();
                (
                    obfuscated.as_str(),
                    ClassMapping {
                        original: original.as_str(),
                        is_synthesized,
                        ..Default::default()
                    },
                )
            })
            .collect();

        for ((obfuscated_class, obfuscated_method), members) in &parsed.members {
            let class_mapping = class_mappings.entry(obfuscated_class.as_str()).or_default();

            let method_mappings = class_mapping
                .members
                .entry(obfuscated_method.as_str())
                .or_default();

            for member in members.all.iter() {
                method_mappings
                    .all_mappings
                    .push(Self::resolve_mapping(&parsed, member));
            }

            for (args, param_members) in members.by_params.iter() {
                let param_mappings = method_mappings.mappings_by_params.entry(args).or_default();

                for member in param_members.iter() {
                    param_mappings.push(Self::resolve_mapping(&parsed, member));
                }
            }
        }

        Self {
            classes: class_mappings,
        }
    }

    fn resolve_mapping(
        parsed: &ParsedProguardMapping<'s>,
        member: &Member<'s>,
    ) -> MemberMapping<'s> {
        let original_file = parsed
            .class_infos
            .get(&member.method.receiver.name())
            .and_then(|class| class.source_file);

        // Only fill in `original_class` if it is _not_ the current class
        let original_class = match member.method.receiver {
            MethodReceiver::ThisClass(_) => None,
            MethodReceiver::OtherClass(original_class_name) => Some(original_class_name.as_str()),
        };

        let method_info = parsed
            .method_infos
            .get(&member.method)
            .copied()
            .unwrap_or_default();
        let is_synthesized = method_info.is_synthesized;
        let is_outline = method_info.is_outline;

        let outline_callsite_positions = member.outline_callsite_positions.clone();

        MemberMapping {
            startline: member.startline,
            endline: member.endline,
            original_class,
            original_file,
            original: member.method.name.as_str(),
            original_startline: member.original_startline,
            original_endline: member.original_endline,
            is_synthesized,
            is_outline,
            outline_callsite_positions,
        }
    }

    /// If the previous frame was an outline and carried a position, attempt to
    /// map that outline position to a callsite position for the given method.
    fn map_outline_position(
        &self,
        class: &str,
        method: &str,
        callsite_line: usize,
        pos: usize,
        parameters: Option<&str>,
    ) -> Option<usize> {
        let ms = self.classes.get(class)?.members.get(method)?;
        let candidates: &[_] = if let Some(params) = parameters {
            match ms.mappings_by_params.get(params) {
                Some(v) => &v[..],
                None => &[],
            }
        } else {
            &ms.all_mappings[..]
        };

        // Find the member mapping covering the callsite line, then map the pos.
        candidates
            .iter()
            .filter(|m| {
                m.endline == 0 || (callsite_line >= m.startline && callsite_line <= m.endline)
            })
            .find_map(|m| {
                m.outline_callsite_positions
                    .as_ref()
                    .and_then(|mm| mm.get(&pos).copied())
            })
    }

    /// Determines if a frame refers to an outline method, either via the
    /// method-level flag or via any matching mapping entry for the frame line.
    fn is_outline_frame(
        &self,
        class: &str,
        method: &str,
        line: usize,
        parameters: Option<&str>,
    ) -> bool {
        self.classes
            .get(class)
            .and_then(|c| c.members.get(method))
            .map(|ms| {
                let mappings: &[_] = if let Some(params) = parameters {
                    match ms.mappings_by_params.get(params) {
                        Some(v) => &v[..],
                        None => &[],
                    }
                } else {
                    &ms.all_mappings[..]
                };
                mappings.iter().any(|m| {
                    m.is_outline && (m.endline == 0 || (line >= m.startline && line <= m.endline))
                })
            })
            .unwrap_or(false)
    }

    /// Applies any carried outline position to the frame line and determines if
    /// the (original) frame is an outline. Returns the adjusted frame and flag.
    fn prepare_frame_for_mapping<'a>(
        &self,
        frame: &StackFrame<'a>,
        carried_outline_pos: &mut Option<usize>,
    ) -> (StackFrame<'a>, bool) {
        let mut effective = frame.clone();
        if let Some(pos) = carried_outline_pos.take() {
            if let Some(mapped) = self.map_outline_position(
                effective.class,
                effective.method,
                effective.line,
                pos,
                effective.parameters,
            ) {
                effective.line = mapped;
            }
        }

        let is_outline =
            self.is_outline_frame(frame.class, frame.method, frame.line, frame.parameters);

        (effective, is_outline)
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

    /// returns a tuple where the first element is the list of the function
    /// parameters and the second one is the return type
    pub fn deobfuscate_signature(&'s self, signature: &str) -> Option<DeobfuscatedSignature> {
        java::deobfuscate_bytecode_signature(signature, self).map(DeobfuscatedSignature::new)
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

        RemappedFrameIter::members(frame, mappings)
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
        let mut carried_outline_pos: Option<usize> = None;

        if let Some(line) = lines.next() {
            match stacktrace::parse_throwable(line) {
                None => match stacktrace::parse_frame(line) {
                    None => writeln!(&mut stacktrace, "{line}")?,
                    Some(frame) => {
                        let (effective_frame, is_outline) =
                            self.prepare_frame_for_mapping(&frame, &mut carried_outline_pos);

                        if is_outline {
                            carried_outline_pos = Some(frame.line);
                        } else {
                            format_frames(
                                &mut stacktrace,
                                line,
                                self.remap_frame(&effective_frame),
                            )?;
                        }
                    }
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
                    None => writeln!(&mut stacktrace, "{line}")?,
                    Some(cause) => {
                        format_cause(&mut stacktrace, line, self.remap_throwable(&cause))?
                    }
                },
                Some(frame) => {
                    let (effective_frame, is_outline) =
                        self.prepare_frame_for_mapping(&frame, &mut carried_outline_pos);

                    if is_outline {
                        carried_outline_pos = Some(frame.line);
                    } else {
                        format_frames(&mut stacktrace, line, self.remap_frame(&effective_frame))?;
                    }
                }
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

        let mut carried_outline_pos: Option<usize> = None;
        let mut frames_out = Vec::with_capacity(trace.frames.len());
        for f in trace.frames.iter() {
            let (effective, is_outline) =
                self.prepare_frame_for_mapping(f, &mut carried_outline_pos);

            if is_outline {
                carried_outline_pos = Some(f.line);
                continue;
            }

            let mut iter = self.remap_frame(&effective).peekable();
            if iter.peek().is_some() {
                frames_out.extend(iter);
            } else {
                frames_out.push(f.clone());
            }
        }

        let cause = trace
            .cause
            .as_ref()
            .map(|c| Box::new(self.remap_stacktrace_typed(c)));

        StackTrace {
            exception,
            frames: frames_out,
            cause,
        }
    }
}

pub(crate) fn format_throwable(
    stacktrace: &mut impl Write,
    line: &str,
    throwable: Option<Throwable<'_>>,
) -> Result<(), FmtError> {
    if let Some(throwable) = throwable {
        writeln!(stacktrace, "{throwable}")
    } else {
        writeln!(stacktrace, "{line}")
    }
}

pub(crate) fn format_frames<'s>(
    stacktrace: &mut impl Write,
    line: &str,
    remapped: impl Iterator<Item = StackFrame<'s>>,
) -> Result<(), FmtError> {
    let mut remapped = remapped.peekable();

    if remapped.peek().is_none() {
        return writeln!(stacktrace, "{line}");
    }
    for line in remapped {
        writeln!(stacktrace, "    {line}")?;
    }

    Ok(())
}

pub(crate) fn format_cause(
    stacktrace: &mut impl Write,
    line: &str,
    cause: Option<Throwable<'_>>,
) -> Result<(), FmtError> {
    if let Some(cause) = cause {
        writeln!(stacktrace, "Caused by: {cause}")
    } else {
        writeln!(stacktrace, "{line}")
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
                    method_synthesized: false,
                },
                StackFrame {
                    class: "android.view.View",
                    method: "performClick",
                    line: 7393,
                    file: Some("View.java"),
                    parameters: None,
                    method_synthesized: false,
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
                    method_synthesized: false,
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
            mapper.remap_stacktrace_typed(&stacktrace).to_string(),
            expect
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

        assert_eq!(mapper.remap_stacktrace(stacktrace).unwrap(), expect);
    }
}
