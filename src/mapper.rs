use std::collections::HashMap;
use std::fmt;
use std::fmt::{Error as FmtError, Write};
use std::iter::FusedIterator;

use crate::builder::{
    Member, MethodReceiver, ParsedProguardMapping, RewriteAction, RewriteCondition, RewriteRule,
};
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
    rewrite_rules: Vec<RewriteRule<'s>>,
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

#[derive(Default)]
struct CollectedFrames<'s> {
    frames: Vec<StackFrame<'s>>,
    rewrite_rules: Vec<&'s RewriteRule<'s>>,
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

fn class_name_to_descriptor(class: &str) -> String {
    let mut descriptor = String::with_capacity(class.len() + 2);
    descriptor.push('L');
    descriptor.push_str(&class.replace('.', "/"));
    descriptor.push(';');
    descriptor
}

fn map_member_with_lines<'a>(
    frame: &StackFrame<'a>,
    member: &MemberMapping<'a>,
) -> Option<StackFrame<'a>> {
    if member.endline > 0 && (frame.line < member.startline || frame.line > member.endline) {
        return None;
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

    let class = member.original_class.unwrap_or(frame.class);

    Some(StackFrame {
        class,
        method: member.original,
        file,
        line,
        parameters: frame.parameters,
        method_synthesized: member.is_synthesized,
    })
}

fn map_member_without_lines<'a>(
    frame: &StackFrame<'a>,
    member: &MemberMapping<'a>,
) -> StackFrame<'a> {
    let class = member.original_class.unwrap_or(frame.class);
    StackFrame {
        class,
        method: member.original,
        file: None,
        line: 0,
        parameters: frame.parameters,
        method_synthesized: member.is_synthesized,
    }
}

fn apply_rewrite_rules<'s>(collected: &mut CollectedFrames<'s>, thrown_descriptor: Option<&str>) {
    for rule in &collected.rewrite_rules {
        let matches = rule.conditions.iter().all(|condition| match condition {
            RewriteCondition::Throws(descriptor) => Some(*descriptor) == thrown_descriptor,
            RewriteCondition::Unknown(_) => false,
        });

        if !matches {
            continue;
        }

        for action in &rule.actions {
            match action {
                RewriteAction::RemoveInnerFrames(count) => {
                    if *count >= collected.frames.len() {
                        collected.frames.clear();
                    } else {
                        collected.frames.drain(0..*count);
                    }
                }
                RewriteAction::Unknown(_) => {}
            }
        }
        if collected.frames.is_empty() {
            break;
        }
    }
}

fn iterate_with_lines<'a>(
    frame: &mut StackFrame<'a>,
    members: &mut core::slice::Iter<'_, MemberMapping<'a>>,
) -> Option<StackFrame<'a>> {
    for member in members {
        if let Some(mapped) = map_member_with_lines(frame, member) {
            return Some(mapped);
        }
    }
    None
}

fn iterate_without_lines<'a>(
    frame: &mut StackFrame<'a>,
    members: &mut core::slice::Iter<'_, MemberMapping<'a>>,
) -> Option<StackFrame<'a>> {
    members
        .next()
        .map(|member| map_member_without_lines(frame, member))
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
            rewrite_rules: member.rewrite_rules.clone(),
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

    /// Determines if a frame refers to an outline method via the method-level flag.
    /// Outline metadata is consistent across all mappings for a method, so checking
    /// a single mapping entry is sufficient.
    fn is_outline_frame(&self, class: &str, method: &str) -> bool {
        self.classes
            .get(class)
            .and_then(|c| c.members.get(method))
            .and_then(|ms| ms.all_mappings.first())
            .is_some_and(|m| m.is_outline)
    }

    /// Applies any carried outline position to the frame line and returns the adjusted frame.
    fn prepare_frame_for_mapping<'a>(
        &self,
        frame: &StackFrame<'a>,
        carried_outline_pos: &mut Option<usize>,
    ) -> StackFrame<'a> {
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

        effective
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

    fn collect_remapped_frames(&'s self, frame: &StackFrame<'s>) -> CollectedFrames<'s> {
        let mut collected = CollectedFrames::default();
        let Some(class) = self.classes.get(frame.class) else {
            return collected;
        };
        let Some(members) = class.members.get(frame.method) else {
            return collected;
        };

        let mut frame = frame.clone();
        frame.class = class.original;

        let mapping_entries: &[MemberMapping<'s>] = if let Some(parameters) = frame.parameters {
            let Some(typed_members) = members.mappings_by_params.get(parameters) else {
                return collected;
            };
            typed_members.as_slice()
        } else {
            members.all_mappings.as_slice()
        };

        if frame.parameters.is_none() {
            for member in mapping_entries {
                if let Some(mapped) = map_member_with_lines(&frame, member) {
                    collected.frames.push(mapped);
                    collected.rewrite_rules.extend(member.rewrite_rules.iter());
                }
            }
        } else {
            for member in mapping_entries {
                let mapped = map_member_without_lines(&frame, member);
                collected.frames.push(mapped);
                collected.rewrite_rules.extend(member.rewrite_rules.iter());
            }
        }

        collected
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
        let mut carried_outline_pos: Option<usize> = None;
        let mut current_exception_descriptor: Option<String> = None;
        let mut next_frame_can_rewrite = false;

        for line in input.lines() {
            if let Some(throwable) = stacktrace::parse_throwable(line) {
                let remapped_throwable = self.remap_throwable(&throwable);
                let descriptor_class = remapped_throwable
                    .as_ref()
                    .map(|t| t.class)
                    .unwrap_or(throwable.class);
                current_exception_descriptor = Some(class_name_to_descriptor(descriptor_class));
                next_frame_can_rewrite = true;
                format_throwable(&mut stacktrace, line, remapped_throwable)?;
                continue;
            }

            if let Some(frame) = stacktrace::parse_frame(line) {
                if self.is_outline_frame(frame.class, frame.method) {
                    carried_outline_pos = Some(frame.line);
                    continue;
                }

                let effective_frame =
                    self.prepare_frame_for_mapping(&frame, &mut carried_outline_pos);

                let mut collected = self.collect_remapped_frames(&effective_frame);
                if !collected.frames.is_empty() {
                    if next_frame_can_rewrite {
                        apply_rewrite_rules(
                            &mut collected,
                            current_exception_descriptor.as_deref(),
                        );
                    }

                    next_frame_can_rewrite = false;
                    current_exception_descriptor = None;

                    if collected.frames.is_empty() {
                        continue;
                    }

                    let drained = collected.frames.drain(..);
                    format_frames(&mut stacktrace, line, drained)?;
                    continue;
                }

                next_frame_can_rewrite = false;
                current_exception_descriptor = None;
                format_frames(&mut stacktrace, line, std::iter::empty())?;
                continue;
            }

            if let Some(cause) = line
                .strip_prefix("Caused by: ")
                .and_then(stacktrace::parse_throwable)
            {
                let remapped_cause = self.remap_throwable(&cause);
                let descriptor_class = remapped_cause
                    .as_ref()
                    .map(|t| t.class)
                    .unwrap_or(cause.class);
                current_exception_descriptor = Some(class_name_to_descriptor(descriptor_class));
                next_frame_can_rewrite = true;
                format_cause(&mut stacktrace, line, remapped_cause)?;
                continue;
            }

            current_exception_descriptor = None;
            next_frame_can_rewrite = false;
            writeln!(&mut stacktrace, "{line}")?;
        }
        Ok(stacktrace)
    }

    /// Remaps a complete Java StackTrace.
    pub fn remap_stacktrace_typed<'a>(&'a self, trace: &StackTrace<'a>) -> StackTrace<'a> {
        let exception = trace
            .exception
            .as_ref()
            .and_then(|t| self.remap_throwable(t));
        let exception_descriptor = trace.exception.as_ref().map(|original| {
            let class = exception
                .as_ref()
                .map(|t| t.class)
                .unwrap_or(original.class);
            class_name_to_descriptor(class)
        });

        let mut carried_outline_pos: Option<usize> = None;
        let mut frames_out = Vec::with_capacity(trace.frames.len());
        let mut next_frame_can_rewrite = exception_descriptor.is_some();
        for f in trace.frames.iter() {
            if self.is_outline_frame(f.class, f.method) {
                carried_outline_pos = Some(f.line);
                continue;
            }

            let effective = self.prepare_frame_for_mapping(f, &mut carried_outline_pos);
            let mut collected = self.collect_remapped_frames(&effective);
            if !collected.frames.is_empty() {
                if next_frame_can_rewrite {
                    apply_rewrite_rules(&mut collected, exception_descriptor.as_deref());
                }
                next_frame_can_rewrite = false;

                if collected.frames.is_empty() {
                    continue;
                }

                frames_out.append(&mut collected.frames);
                continue;
            }

            next_frame_can_rewrite = false;
            frames_out.push(f.clone());
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

    #[test]
    fn rewrite_frame_remove_inner_frame() {
        let mapping = "\
some.Class -> a:
    4:4:void other.Class.inlinee():23:23 -> a
    4:4:void caller(other.Class):7 -> a
    # {\"id\":\"com.android.tools.r8.rewriteFrame\",\"conditions\":[\"throws(Ljava/lang/NullPointerException;)\"],\"actions\":[\"removeInnerFrames(1)\"]}
";
        let stacktrace = "\
java.lang.NullPointerException: Boom
    at a.a(SourceFile:4)";
        let expect = "\
java.lang.NullPointerException: Boom
    at some.Class.caller(SourceFile:7)
";

        let mapper = ProguardMapper::from(mapping);

        assert_eq!(mapper.remap_stacktrace(stacktrace).unwrap(), expect);
    }

    #[test]
    fn rewrite_frame_condition_mismatch() {
        let mapping = "\
some.Class -> a:
    4:4:void other.Class.inlinee():23:23 -> a
    4:4:void caller(other.Class):7 -> a
    # {\"id\":\"com.android.tools.r8.rewriteFrame\",\"conditions\":[\"throws(Ljava/lang/NullPointerException;)\"],\"actions\":[\"removeInnerFrames(1)\"]}
";
        let stacktrace = "\
java.lang.IllegalStateException: Boom
    at a.a(SourceFile:4)";
        let expect = "\
java.lang.IllegalStateException: Boom
    at other.Class.inlinee(<unknown>:23)
    at some.Class.caller(SourceFile:7)
";

        let mapper = ProguardMapper::from(mapping);

        assert_eq!(mapper.remap_stacktrace(stacktrace).unwrap(), expect);
    }

    #[test]
    fn rewrite_frame_typed_stacktrace() {
        let mapping = "\
some.Class -> a:
    4:4:void other.Class.inlinee():23:23 -> a
    4:4:void caller(other.Class):7 -> a
    # {\"id\":\"com.android.tools.r8.rewriteFrame\",\"conditions\":[\"throws(Ljava/lang/NullPointerException;)\"],\"actions\":[\"removeInnerFrames(1)\"]}
";
        let trace = StackTrace {
            exception: Some(Throwable {
                class: "java.lang.NullPointerException",
                message: Some("Boom"),
            }),
            frames: vec![StackFrame {
                class: "a",
                method: "a",
                line: 4,
                file: Some("SourceFile"),
                parameters: None,
                method_synthesized: false,
            }],
            cause: None,
        };

        let mapper = ProguardMapper::from(mapping);
        let remapped = mapper.remap_stacktrace_typed(&trace);

        assert_eq!(remapped.frames.len(), 1);
        assert_eq!(remapped.frames[0].class, "some.Class");
        assert_eq!(remapped.frames[0].method, "caller");
        assert_eq!(remapped.frames[0].line, 7);
    }

    #[test]
    fn rewrite_frame_multiple_rules_or_semantics() {
        let mapping = "\
some.Class -> a:
    4:4:void other.Class.inlinee():23:23 -> call
    4:4:void outer():7 -> call
    # {\"id\":\"com.android.tools.r8.rewriteFrame\",\"conditions\":[\"throws(Ljava/lang/NullPointerException;)\"],\"actions\":[\"removeInnerFrames(1)\"]}
    # {\"id\":\"com.android.tools.r8.rewriteFrame\",\"conditions\":[\"throws(Ljava/lang/IllegalStateException;)\"],\"actions\":[\"removeInnerFrames(1)\"]}
";
        let mapper = ProguardMapper::from(mapping);

        let input_npe = "\
java.lang.NullPointerException: Boom
    at a.call(SourceFile:4)";
        let expected_npe = "\
java.lang.NullPointerException: Boom
    at some.Class.outer(SourceFile:7)
";
        assert_eq!(mapper.remap_stacktrace(input_npe).unwrap(), expected_npe);

        let input_ise = "\
java.lang.IllegalStateException: Boom
    at a.call(SourceFile:4)";
        let expected_ise = "\
java.lang.IllegalStateException: Boom
    at some.Class.outer(SourceFile:7)
";
        assert_eq!(mapper.remap_stacktrace(input_ise).unwrap(), expected_ise);
    }

    #[test]
    fn remap_frame_without_mapping_keeps_original_line() {
        let mapping = "\
some.Class -> a:
    1:1:void some.Class.existing():10:10 -> a
";
        let mapper = ProguardMapper::from(mapping);

        let input = "\
java.lang.RuntimeException: boom
    at a.missing(SourceFile:42)
";
        let expected = "\
java.lang.RuntimeException: boom
    at a.missing(SourceFile:42)
";

        assert_eq!(mapper.remap_stacktrace(input).unwrap(), expected);
    }
}
