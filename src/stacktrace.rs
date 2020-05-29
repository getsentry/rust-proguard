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
    pub(crate) line: usize,
    pub(crate) file: Option<Cow<'s, str>>,
}

impl<'s> StackFrame<'s> {
    /// Create a new StackFrame.
    pub fn new(class: &'s str, method: &'s str, line: usize) -> Self {
        Self {
            class: class.into(),
            method: method.into(),
            line,
            file: None,
        }
    }

    /// Create a new StackFrame with file information.
    pub fn with_file(class: &'s str, method: &'s str, line: usize, file: &'s str) -> Self {
        Self {
            class: class.into(),
            method: method.into(),
            line,
            file: Some(file.into()),
        }
    }

    /// Parses a StackFrame from a line of a Java StackTrace.
    ///
    /// # Examples
    ///
    /// ```
    /// use proguard::StackFrame;
    ///
    /// let parsed = StackFrame::try_parse(b"    at some.Klass.method(Klass.java:1234)");
    /// assert_eq!(
    ///     parsed,
    ///     Some(StackFrame::with_file(
    ///         "some.Klass",
    ///         "method",
    ///         1234,
    ///         "Klass.java"
    ///     ))
    /// );
    /// ```
    pub fn try_parse(line: &'s [u8]) -> Option<Self> {
        let line = std::str::from_utf8(line).ok()?;
        parse_frame(line)
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
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    /// The line of the StackFrame.
    pub fn line(&self) -> usize {
        self.line
    }
}

/// Parses a single line from a Java StackTrace.
///
/// Returns [`None`] if the line could not be parsed.
fn parse_frame(line: &str) -> Option<StackFrame> {
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
        file: Some(file.into()),
        line,
    })
}
