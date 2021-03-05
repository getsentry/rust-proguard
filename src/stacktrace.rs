//! A Parser for Java Stacktraces.

use std::fmt::{Display, Formatter, Result as FmtResult};

/// A full Java StackTrace as printed by [`Throwable.printStackTrace()`].
///
/// [`Throwable.printStackTrace()`]: https://docs.oracle.com/en/java/javase/14/docs/api/java.base/java/lang/Throwable.html#printStackTrace()
#[derive(Clone, Debug, PartialEq)]
pub struct StackTrace<'s> {
    pub(crate) exception: Option<Throwable<'s>>,
    pub(crate) frames: Vec<StackFrame<'s>>,
    pub(crate) cause: Option<Box<StackTrace<'s>>>,
}

impl<'s> StackTrace<'s> {
    /// Create a new StackTrace.
    pub fn new(exception: Option<Throwable<'s>>, frames: Vec<StackFrame<'s>>) -> Self {
        Self {
            exception,
            frames,
            cause: None,
        }
    }

    /// Create a new StackTrace with cause information.
    pub fn with_cause(
        exception: Option<Throwable<'s>>,
        frames: Vec<StackFrame<'s>>,
        cause: StackTrace<'s>,
    ) -> Self {
        Self {
            exception,
            frames,
            cause: Some(Box::new(cause)),
        }
    }

    /// Parses a StackTrace from a full Java StackTrace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use proguard::{StackFrame, StackTrace, Throwable};
    ///
    /// let stacktrace = "\
    /// some.CustomException: Crashed!
    ///     at some.Klass.method(Klass.java:1234)
    /// Caused by: some.InnerException
    ///     at some.Klass2.method2(Klass2.java:5678)
    /// ";
    /// let parsed = StackTrace::try_parse(stacktrace.as_bytes());
    /// assert_eq!(
    ///     parsed,
    ///     Some(StackTrace::with_cause(
    ///         Some(Throwable::with_message("some.CustomException", "Crashed!")),
    ///         vec![StackFrame::with_file(
    ///             "some.Klass",
    ///             "method",
    ///             1234,
    ///             "Klass.java",
    ///         )],
    ///         StackTrace::new(
    ///             Some(Throwable::new("some.InnerException")),
    ///             vec![StackFrame::with_file(
    ///                 "some.Klass2",
    ///                 "method2",
    ///                 5678,
    ///                 "Klass2.java",
    ///             )]
    ///         )
    ///     ))
    /// );
    /// ```
    pub fn try_parse(stacktrace: &'s [u8]) -> Option<Self> {
        let stacktrace = std::str::from_utf8(stacktrace).ok()?;
        parse_stacktrace(stacktrace)
    }

    /// The exception at the top of the StackTrace, if present.
    pub fn exception(&self) -> Option<&Throwable<'_>> {
        self.exception.as_ref()
    }

    /// All StackFrames following the exception.
    pub fn frames(&self) -> &[StackFrame<'_>] {
        &self.frames
    }

    /// An optional cause describing the inner exception.
    pub fn cause(&self) -> Option<&StackTrace<'_>> {
        self.cause.as_deref()
    }
}

impl<'s> Display for StackTrace<'s> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if let Some(exception) = &self.exception {
            writeln!(f, "{}", exception)?;
        }

        for frame in &self.frames {
            writeln!(f, "    {}", frame)?;
        }

        if let Some(cause) = &self.cause {
            write!(f, "Caused by: {}", cause)?;
        }

        Ok(())
    }
}

fn parse_stacktrace(content: &str) -> Option<StackTrace<'_>> {
    let mut lines = content.lines().peekable();

    let exception = lines.peek().and_then(|line| parse_throwable(line));
    if exception.is_some() {
        lines.next();
    }

    let mut stacktrace = StackTrace {
        exception,
        frames: vec![],
        cause: None,
    };
    let mut current = &mut stacktrace;

    for line in &mut lines {
        if let Some(frame) = parse_frame(line) {
            current.frames.push(frame);
        } else if let Some(line) = line.strip_prefix("Caused by: ") {
            current.cause = Some(Box::new(StackTrace {
                exception: parse_throwable(line),
                frames: vec![],
                cause: None,
            }));
            // We just set the `cause` so it's safe to unwrap here
            current = current.cause.as_deref_mut().unwrap();
        }
    }

    if stacktrace.exception.is_some() || !stacktrace.frames.is_empty() {
        Some(stacktrace)
    } else {
        None
    }
}

/// A Java StackFrame.
///
/// Basically a Rust version of the Java [`StackTraceElement`].
///
/// [`StackTraceElement`]: https://docs.oracle.com/en/java/javase/14/docs/api/java.base/java/lang/StackTraceElement.html
#[derive(Clone, Debug, PartialEq)]
pub struct StackFrame<'s> {
    pub(crate) class: &'s str,
    pub(crate) method: &'s str,
    pub(crate) line: usize,
    pub(crate) file: Option<&'s str>,
}

impl<'s> StackFrame<'s> {
    /// Create a new StackFrame.
    pub fn new(class: &'s str, method: &'s str, line: usize) -> Self {
        Self {
            class,
            method,
            line,
            file: None,
        }
    }

    /// Create a new StackFrame with file information.
    pub fn with_file(class: &'s str, method: &'s str, line: usize, file: &'s str) -> Self {
        Self {
            class,
            method,
            line,
            file: Some(file),
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
        self.class
    }

    /// The method of the StackFrame.
    pub fn method(&self) -> &str {
        self.method
    }

    /// The fully qualified method name, including the class.
    pub fn full_method(&self) -> String {
        format!("{}.{}", self.class, self.method)
    }

    /// The file of the StackFrame.
    pub fn file(&self) -> Option<&str> {
        self.file
    }

    /// The line of the StackFrame, 1-based.
    pub fn line(&self) -> usize {
        self.line
    }
}

impl<'s> Display for StackFrame<'s> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "at {}.{}({}:{})",
            self.class,
            self.method,
            self.file.unwrap_or("<unknown>"),
            self.line
        )
    }
}

/// Parses a single line from a Java StackTrace.
///
/// Returns `None` if the line could not be parsed.
pub(crate) fn parse_frame(line: &str) -> Option<StackFrame> {
    let line = line.trim();

    if !line.starts_with("at ") || !line.ends_with(')') {
        return None;
    }
    let mut arg_split = line[3..line.len() - 1].splitn(2, '(');

    let mut method_split = arg_split.next()?.rsplitn(2, '.');
    let method = method_split.next()?;
    let class = method_split.next()?;

    let mut file_split = arg_split.next()?.splitn(2, ':');
    let file = file_split.next()?;
    let line = file_split.next()?.parse().ok()?;

    Some(StackFrame {
        class,
        method,
        file: Some(file),
        line,
    })
}

/// A Java Throwable.
///
/// This is a Rust version of the first line from a [`Throwable.printStackTrace()`] output in Java.
///
/// [`Throwable.printStackTrace()`]: https://docs.oracle.com/en/java/javase/14/docs/api/java.base/java/lang/Throwable.html#printStackTrace()
#[derive(Clone, Debug, PartialEq)]
pub struct Throwable<'s> {
    pub(crate) class: &'s str,
    pub(crate) message: Option<&'s str>,
}

impl<'s> Throwable<'s> {
    /// Create a new Throwable.
    pub fn new(class: &'s str) -> Self {
        Self {
            class,
            message: None,
        }
    }

    /// Create a new Throwable with message.
    pub fn with_message(class: &'s str, message: &'s str) -> Self {
        Self {
            class,
            message: Some(message),
        }
    }

    /// Parses a Throwable from the a line of a full Java StackTrace.
    ///
    /// # Example
    /// ```rust
    /// use proguard::Throwable;
    ///
    /// let parsed = Throwable::try_parse(b"some.CustomException: Crash!");
    /// assert_eq!(
    ///     parsed,
    ///     Some(Throwable::with_message("some.CustomException", "Crash!")),
    /// )
    /// ```
    pub fn try_parse(line: &'s [u8]) -> Option<Self> {
        std::str::from_utf8(line).ok().and_then(parse_throwable)
    }

    /// The class of this Throwable.
    pub fn class(&self) -> &str {
        self.class
    }

    /// The optional message of this Throwable.
    pub fn message(&self) -> Option<&str> {
        self.message
    }
}

impl<'s> Display for Throwable<'s> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.class)?;

        if let Some(message) = self.message {
            write!(f, ": {}", message)?;
        }

        Ok(())
    }
}

/// Parse the first line of a Java StackTrace which is usually the string version of a
/// [`Throwable`].
///
/// Returns `None` if the line could not be parsed.
///
/// [`Throwable`]: https://docs.oracle.com/en/java/javase/14/docs/api/java.base/java/lang/Throwable.html
pub(crate) fn parse_throwable(line: &str) -> Option<Throwable<'_>> {
    let line = line.trim();

    let mut class_split = line.splitn(2, ": ");
    let class = class_split.next()?;
    let message = class_split.next();

    if class.contains(' ') {
        None
    } else {
        Some(Throwable { class, message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_stack_trace() {
        let trace = StackTrace {
            exception: Some(Throwable {
                class: "com.example.MainFragment",
                message: Some("Crash"),
            }),
            frames: vec![StackFrame {
                class: "com.example.Util",
                method: "show",
                line: 5,
                file: Some("Util.java"),
            }],
            cause: Some(Box::new(StackTrace {
                exception: Some(Throwable {
                    class: "com.example.Other",
                    message: Some("Invalid data"),
                }),
                frames: vec![StackFrame {
                    class: "com.example.Parser",
                    method: "parse",
                    line: 115,
                    file: None,
                }],
                cause: None,
            })),
        };
        let expect = "\
com.example.MainFragment: Crash
    at com.example.Util.show(Util.java:5)
Caused by: com.example.Other: Invalid data
    at com.example.Parser.parse(<unknown>:115)\n";

        assert_eq!(expect, trace.to_string());
    }

    #[test]
    fn stack_frame() {
        let line = "at com.example.MainFragment.onClick(SourceFile:1)";
        let stack_frame = parse_frame(line);
        let expect = Some(StackFrame {
            class: "com.example.MainFragment",
            method: "onClick",
            line: 1,
            file: Some("SourceFile"),
        });

        assert_eq!(expect, stack_frame);

        let line = "    at com.example.MainFragment.onClick(SourceFile:1)";
        let stack_frame = parse_frame(line);

        assert_eq!(expect, stack_frame);

        let line = "\tat com.example.MainFragment.onClick(SourceFile:1)";
        let stack_frame = parse_frame(line);

        assert_eq!(expect, stack_frame);
    }

    #[test]
    fn print_stack_frame() {
        let frame = StackFrame {
            class: "com.example.MainFragment",
            method: "onClick",
            line: 1,
            file: None,
        };

        assert_eq!(
            "at com.example.MainFragment.onClick(<unknown>:1)",
            frame.to_string()
        );

        let frame = StackFrame {
            class: "com.example.MainFragment",
            method: "onClick",
            line: 1,
            file: Some("SourceFile"),
        };

        assert_eq!(
            "at com.example.MainFragment.onClick(SourceFile:1)",
            frame.to_string()
        );
    }

    #[test]
    fn throwable() {
        let line = "com.example.MainFragment: Crash!";
        let throwable = parse_throwable(line);
        let expect = Some(Throwable {
            class: "com.example.MainFragment",
            message: Some("Crash!"),
        });

        assert_eq!(expect, throwable);
    }

    #[test]
    fn print_throwable() {
        let throwable = Throwable {
            class: "com.example.MainFragment",
            message: None,
        };

        assert_eq!("com.example.MainFragment", throwable.to_string());

        let throwable = Throwable {
            class: "com.example.MainFragment",
            message: Some("Crash"),
        };

        assert_eq!("com.example.MainFragment: Crash", throwable.to_string());
    }
}
