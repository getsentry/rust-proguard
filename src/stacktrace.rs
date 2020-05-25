//! A Parser for Java Stacktraces.

use std::borrow::Cow;

/// A Java StackFrame.
///
/// Basically a Rust version of the Java [`StackTraceElement`].
///
/// [`StackTraceElement`]: https://docs.oracle.com/en/java/javase/14/docs/api/java.base/java/lang/StackTraceElement.html
#[derive(Clone, Debug, PartialEq)]
pub struct StackFrame<'s> {
    pub(crate) class: Cow<'s, str>,
    pub(crate) method: Cow<'s, str>,
    pub(crate) file: Cow<'s, str>,
    pub(crate) line: usize,
}

impl<'s> StackFrame<'s> {
    /// Create a new StackFrame.
    pub fn new(class: &'s str, method: &'s str, file: &'s str, line: usize) -> Self {
        Self {
            class: class.into(),
            method: method.into(),
            file: file.into(),
            line,
        }
    }

    /// The class of the StackFrame.
    pub fn class(&self) -> &str {
        self.class.as_ref()
    }

    /// The method of the StackFrame.
    pub fn method(&self) -> &str {
        self.method.as_ref()
    }

    /// The file of the StackFrame.
    pub fn file(&self) -> &str {
        self.file.as_ref()
    }

    /// The line of the StackFrame.
    pub fn line(&self) -> usize {
        self.line
    }
}

/// Parses a single line from a Java StackTrace.
///
/// Returns [`None`] if the line could not be parsed.
pub fn parse_stacktrace_line(line: &str) -> Option<StackFrame> {
    if !line.starts_with("    at ") || !line.ends_with(')') {
        return None;
    }
    let mut arg_split = line[7..line.len() - 1].splitn(2, '(');

    let mut method_split = arg_split.next()?.rsplitn(2, '.');
    let method = method_split.next()?;
    let class = method_split.next()?;

    let mut file_split = arg_split.next()?.splitn(2, ':');
    let file = file_split.next()?;
    let line = file_split.next()?.parse().ok()?;

    Some(StackFrame {
        class: class.into(),
        method: method.into(),
        file: file.into(),
        line,
    })
}
