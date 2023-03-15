//! A Parser for Proguard Mapping Files.
//!
//! The mapping file format is described
//! [here](https://www.guardsquare.com/en/products/proguard/manual/retrace).

use std::fmt;
use std::str;

#[cfg(feature = "uuid")]
use uuid_::Uuid;

/// Error when parsing a proguard mapping line.
///
/// Since the mapping parses proguard line-by-line, an error will also contain
/// the offending line.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParseError<'s> {
    line: &'s [u8],
    kind: ParseErrorKind,
}

impl<'s> ParseError<'s> {
    /// The offending line that caused the error.
    pub fn line(&self) -> &[u8] {
        self.line
    }

    /// The specific parse Error.
    pub fn kind(&self) -> ParseErrorKind {
        self.kind
    }
}

impl fmt::Display for ParseError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ParseErrorKind::Utf8Error(e) => e.fmt(f),
            ParseErrorKind::ParseError(d) => d.fmt(f),
        }
    }
}

impl std::error::Error for ParseError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.kind {
            ParseErrorKind::Utf8Error(ref e) => Some(e),
            ParseErrorKind::ParseError(_) => None,
        }
    }
}

/// The specific parse Error.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ParseErrorKind {
    /// The line failed utf-8 conversion.
    Utf8Error(str::Utf8Error),
    /// The line failed parsing.
    ParseError(&'static str),
}

/// Summary of a mapping file.
pub struct MappingSummary<'s> {
    compiler: Option<&'s str>,
    compiler_version: Option<&'s str>,
    min_api: Option<u32>,
    class_count: usize,
    method_count: usize,
}

impl<'s> MappingSummary<'s> {
    fn new(mapping: &'s ProguardMapping<'s>) -> MappingSummary<'s> {
        let mut compiler = None;
        let mut compiler_version = None;
        let mut min_api = None;
        let mut class_count = 0;
        let mut method_count = 0;

        for record in mapping.iter() {
            match record {
                Ok(ProguardRecord::Header { key, value }) => match key {
                    "compiler" => {
                        compiler = value;
                    }
                    "compiler_version" => {
                        compiler_version = value;
                    }
                    "min_api" => {
                        min_api = value.and_then(|x| x.parse().ok());
                    }
                    _ => {}
                },
                Ok(ProguardRecord::Class { .. }) => class_count += 1,
                Ok(ProguardRecord::Method { .. }) => method_count += 1,
                _ => {}
            }
        }

        MappingSummary {
            compiler,
            compiler_version,
            min_api,
            class_count,
            method_count,
        }
    }

    /// Returns the name of the compiler that created the proguard mapping.
    pub fn compiler(&self) -> Option<&str> {
        self.compiler
    }

    /// Returns the version of the compiler.
    pub fn compiler_version(&self) -> Option<&str> {
        self.compiler_version
    }

    /// Returns the min-api value.
    pub fn min_api(&self) -> Option<u32> {
        self.min_api
    }

    /// Returns the number of classes in the mapping file.
    pub fn class_count(&self) -> usize {
        self.class_count
    }

    /// Returns the number of methods in the mapping file.
    pub fn method_count(&self) -> usize {
        self.method_count
    }
}

/// A Proguard Mapping file.
#[derive(Clone, Default)]
pub struct ProguardMapping<'s> {
    source: &'s [u8],
}

impl<'s> fmt::Debug for ProguardMapping<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProguardMapping").finish()
    }
}

impl<'s> ProguardMapping<'s> {
    /// Create a new Proguard Mapping.
    pub fn new(source: &'s [u8]) -> Self {
        Self { source }
    }

    /// Whether the mapping file is indeed valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use proguard::ProguardMapping;
    ///
    /// let valid = ProguardMapping::new(b"a -> b:\n    void method() -> b");
    /// assert_eq!(valid.is_valid(), true);
    ///
    /// let invalid = ProguardMapping::new(
    ///     br#"
    /// # looks: like
    /// a -> proguard:
    ///   mapping but(is) -> not
    /// "#,
    /// );
    /// assert_eq!(invalid.is_valid(), false);
    /// ```
    pub fn is_valid(&self) -> bool {
        // In order to not parse the whole file, we look for a class followed by
        // a member in the first 50 lines, which is a good heuristic.
        let mut has_class_line = false;
        for record in self.iter().take(50) {
            match record {
                Ok(ProguardRecord::Class { .. }) => {
                    has_class_line = true;
                }
                Ok(ProguardRecord::Field { .. }) | Ok(ProguardRecord::Method { .. })
                    if has_class_line =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// Returns a summary of the file.
    pub fn summary(&self) -> MappingSummary<'_> {
        MappingSummary::new(self)
    }

    /// Whether the mapping file contains line info.
    ///
    /// # Examples
    ///
    /// ```
    /// use proguard::ProguardMapping;
    ///
    /// let with = ProguardMapping::new(b"a -> b:\n    1:1:void method() -> a");
    /// assert_eq!(with.has_line_info(), true);
    ///
    /// let without = ProguardMapping::new(b"a -> b:\n    void method() -> b");
    /// assert_eq!(without.has_line_info(), false);
    /// ```
    pub fn has_line_info(&self) -> bool {
        // We are matching on the inner `ProguardRecord` anyway
        #[allow(clippy::manual_flatten)]
        for record in self.iter() {
            if let Ok(ProguardRecord::Method { line_mapping, .. }) = record {
                if line_mapping.is_some() {
                    return true;
                }
            }
        }
        false
    }

    /// Calculates the UUID of the mapping file.
    ///
    /// The UUID is generated from a file checksum.
    #[cfg(feature = "uuid")]
    pub fn uuid(&self) -> Uuid {
        lazy_static::lazy_static! {
            static ref NAMESPACE: Uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"guardsquare.com");
        }
        // this internally only operates on bytes, so this is safe to do
        Uuid::new_v5(&NAMESPACE, self.source)
    }

    /// Create an Iterator over [`ProguardRecord`]s.
    ///
    /// [`ProguardRecord`]: enum.ProguardRecord.html
    pub fn iter(&self) -> ProguardRecordIter<'s> {
        ProguardRecordIter { slice: self.source }
    }
}

/// An Iterator yielding [`ProguardRecord`]s, created by [`ProguardMapping::iter`].
///
/// [`ProguardRecord`]: enum.ProguardRecord.html
/// [`ProguardMapping::iter`]: struct.ProguardMapping.html#method.iter
#[derive(Clone, Default)]
pub struct ProguardRecordIter<'s> {
    slice: &'s [u8],
}

impl<'s> fmt::Debug for ProguardRecordIter<'s> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProguardRecordIter").finish()
    }
}

impl<'s> Iterator for ProguardRecordIter<'s> {
    type Item = Result<ProguardRecord<'s>, ParseError<'s>>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        let (result, slice) = parse_proguard_record(self.slice);
        self.slice = slice;
        Some(result)
    }
}

/// A proguard line mapping.
///
/// Maps start/end lines of a minified file to original start/end lines.
///
/// All line mappings are 1-based and inclusive.
#[derive(Clone, Debug, PartialEq)]
pub struct LineMapping {
    /// Start Line, 1-based.
    pub startline: usize,
    /// End Line, inclusive.
    pub endline: usize,
    /// The original Start Line.
    pub original_startline: Option<usize>,
    /// The original End Line.
    pub original_endline: Option<usize>,
}

/// A Proguard Mapping Record.
#[derive(Clone, Debug, PartialEq)]
pub enum ProguardRecord<'s> {
    /// A Proguard Header.
    Header {
        /// The Key of the Header.
        key: &'s str,
        /// Optional value if the Header is a KV pair.
        value: Option<&'s str>,
    },
    /// A Class Mapping.
    Class {
        /// Original name of the class.
        original: &'s str,
        /// Obfuscated name of the class.
        obfuscated: &'s str,
    },
    /// A Field Mapping.
    Field {
        /// Type of the field
        ty: &'s str,
        /// Original name of the field.
        original: &'s str,
        /// Obfuscated name of the field.
        obfuscated: &'s str,
    },
    /// A Method Mapping.
    Method {
        /// Return Type of the method.
        ty: &'s str,
        /// Original name of the method.
        original: &'s str,
        /// Obfuscated name of the method.
        obfuscated: &'s str,
        /// Arguments of the method as raw string.
        arguments: &'s str,
        /// Original class of a foreign inlined method.
        original_class: Option<&'s str>,
        /// Optional line mapping of the method.
        line_mapping: Option<LineMapping>,
    },
}

impl<'s> ProguardRecord<'s> {
    /// Parses a line from a proguard mapping file.
    ///
    /// # Examples
    ///
    /// ```
    /// use proguard::ProguardRecord;
    ///
    /// // Headers
    /// let parsed = ProguardRecord::try_parse(b"# compiler: R8");
    /// assert_eq!(
    ///     parsed,
    ///     Ok(ProguardRecord::Header {
    ///         key: "compiler",
    ///         value: Some("R8")
    ///     })
    /// );
    ///
    /// // Class Mappings
    /// let parsed =
    ///     ProguardRecord::try_parse(b"android.arch.core.executor.ArchTaskExecutor -> a.a.a.a.c:");
    /// assert_eq!(
    ///     parsed,
    ///     Ok(ProguardRecord::Class {
    ///         original: "android.arch.core.executor.ArchTaskExecutor",
    ///         obfuscated: "a.a.a.a.c"
    ///     })
    /// );
    ///
    /// // Field
    /// let parsed = ProguardRecord::try_parse(
    ///     b"    android.arch.core.executor.ArchTaskExecutor sInstance -> a",
    /// );
    /// assert_eq!(
    ///     parsed,
    ///     Ok(ProguardRecord::Field {
    ///         ty: "android.arch.core.executor.ArchTaskExecutor",
    ///         original: "sInstance",
    ///         obfuscated: "a",
    ///     })
    /// );
    ///
    /// // Method without line mappings
    /// let parsed = ProguardRecord::try_parse(
    ///     b"    java.lang.Object putIfAbsent(java.lang.Object,java.lang.Object) -> b",
    /// );
    /// assert_eq!(
    ///     parsed,
    ///     Ok(ProguardRecord::Method {
    ///         ty: "java.lang.Object",
    ///         original: "putIfAbsent",
    ///         obfuscated: "b",
    ///         arguments: "java.lang.Object,java.lang.Object",
    ///         original_class: None,
    ///         line_mapping: None,
    ///     })
    /// );
    ///
    /// // Inlined method from foreign class
    /// let parsed = ProguardRecord::try_parse(
    ///     b"    1016:1016:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked",
    /// );
    /// assert_eq!(
    ///     parsed,
    ///     Ok(ProguardRecord::Method {
    ///         ty: "void",
    ///         original: "doWork",
    ///         obfuscated: "buttonClicked",
    ///         arguments: "",
    ///         original_class: Some("com.example1.domain.MyBean"),
    ///         line_mapping: Some(proguard::LineMapping {
    ///             startline: 1016,
    ///             endline: 1016,
    ///             original_startline: Some(16),
    ///             original_endline: Some(16),
    ///         }),
    ///     })
    /// );
    /// ```
    pub fn try_parse(line: &'s [u8]) -> Result<Self, ParseError<'s>> {
        match parse_proguard_record(line) {
            (Err(err), _) => Err(err),
            // We were able to extract a record from the line but there are bytes remaining
            // when they should have all been consumed during parsing
            (Ok(_), slice) if !slice.is_empty() => Err(ParseError {
                line,
                kind: ParseErrorKind::ParseError("line is not a valid proguard record"),
            }),
            (Ok(record), _) => Ok(record),
        }
    }
}

/// Parses a single line from a Proguard File.
///
/// Returns `Err(ParseError)` if the line could not be parsed.
fn parse_proguard_record(bytes: &[u8]) -> (Result<ProguardRecord, ParseError>, &[u8]) {
    let bytes = consume_leading_newlines(bytes);

    let result = if bytes.starts_with(b"#") {
        parse_proguard_header(bytes)
    } else if bytes.starts_with(b"    ") {
        parse_proguard_field_or_method(bytes)
    } else {
        parse_proguard_class(bytes)
    };

    match result {
        Ok((record, bytes)) => (Ok(record), bytes),
        Err(_) => {
            let (line, bytes) = split_line(bytes);
            (
                Err(ParseError {
                    line,
                    kind: ParseErrorKind::ParseError("line is not a valid proguard record"),
                }),
                bytes,
            )
        }
    }
}

/// Parses a single Proguard Header from a Proguard File.
fn parse_proguard_header(bytes: &[u8]) -> Result<(ProguardRecord, &[u8]), ParseError> {
    let bytes = parse_prefix(bytes, b"#")?;

    let (key, bytes) = parse_until_or_newline(bytes, |c| *c == b':')?;

    let (value, bytes) = match parse_prefix(bytes, b":") {
        Ok(bytes) => parse_until(bytes, is_newline).map(|(v, bytes)| (Some(v), bytes)),
        Err(_) => Ok((None, bytes)),
    }?;

    let record = ProguardRecord::Header {
        key: key.trim(),
        value: value.map(|v| v.trim()),
    };

    Ok((record, consume_leading_newlines(bytes)))
}

/// Parses a single Proguard Field or Method from a Proguard File.
fn parse_proguard_field_or_method(bytes: &[u8]) -> Result<(ProguardRecord, &[u8]), ParseError> {
    // field line or method line:
    // `originalfieldtype originalfieldname -> obfuscatedfieldname`
    // `[startline:endline:]originalreturntype [originalclassname.]originalmethodname(originalargumenttype,...)[:originalstartline[:originalendline]] -> obfuscatedmethodname`
    let bytes = parse_prefix(bytes, b"    ")?;

    let (startline, bytes) = match parse_usize(bytes) {
        Ok((startline, bytes)) => (Some(startline), bytes),
        Err(_) => (None, bytes),
    };

    let (endline, bytes) = match startline {
        Some(_) => {
            let bytes = parse_prefix(bytes, b":")?;
            let (endline, bytes) = parse_usize(bytes)?;
            let bytes = parse_prefix(bytes, b":")?;
            (Some(endline), bytes)
        }
        None => (None, bytes),
    };

    let (ty, bytes) = parse_until(bytes, |c| *c == b' ')?;

    let bytes = parse_prefix(bytes, b" ")?;

    let (original, bytes) = parse_until(bytes, |c| *c == b' ' || *c == b'(')?;

    let (arguments, bytes) = match parse_prefix(bytes, b"(") {
        Ok(bytes) => {
            let (arguments, bytes) = parse_until(bytes, |c| *c == b')')?;
            let bytes = parse_prefix(bytes, b")")?;
            (Some(arguments), bytes)
        }
        Err(_) => (None, bytes),
    };

    let (original_startline, bytes) = match arguments {
        Some(_) => match parse_prefix(bytes, b":") {
            Ok(bytes) => {
                let (original_startline, bytes) = parse_usize(bytes)?;
                (Some(original_startline), bytes)
            }
            Err(_) => (None, bytes),
        },
        None => (None, bytes),
    };

    let (original_endline, bytes) = match original_startline {
        Some(_) => match parse_prefix(bytes, b":") {
            Ok(bytes) => {
                let (original_endline, bytes) = parse_usize(bytes)?;
                (Some(original_endline), bytes)
            }
            Err(_) => (None, bytes),
        },
        None => (None, bytes),
    };

    let bytes = parse_prefix(bytes, b" -> ")?;

    let (obfuscated, bytes) = parse_until(bytes, is_newline)?;

    let record = match arguments {
        Some(arguments) => {
            let mut split_class = original.rsplitn(2, '.');
            let original = split_class.next().ok_or(ParseError {
                line: bytes,
                kind: ParseErrorKind::ParseError("line is not a valid proguard record"),
            })?;
            let original_class = split_class.next();

            let line_mapping = match (startline, endline) {
                (Some(startline), Some(endline)) if startline > 0 && endline > 0 => {
                    Some(LineMapping {
                        startline,
                        endline,
                        original_startline,
                        original_endline,
                    })
                }
                _ => None,
            };

            ProguardRecord::Method {
                ty,
                original,
                obfuscated,
                arguments,
                original_class,
                line_mapping,
            }
        }
        None => ProguardRecord::Field {
            ty,
            original,
            obfuscated,
        },
    };

    Ok((record, consume_leading_newlines(bytes)))
}

/// Parses a single Proguard Class from a Proguard File.
fn parse_proguard_class(bytes: &[u8]) -> Result<(ProguardRecord, &[u8]), ParseError> {
    // class line:
    // `originalclassname -> obfuscatedclassname:`
    let (original, bytes) = parse_until(bytes, |c| *c == b' ')?;

    let bytes = parse_prefix(bytes, b" -> ")?;

    let (obfuscated, bytes) = parse_until(bytes, |c| *c == b':')?;

    let bytes = parse_prefix(bytes, b":")?;

    let record = ProguardRecord::Class {
        original,
        obfuscated,
    };

    Ok((record, consume_leading_newlines(bytes)))
}

fn parse_usize(bytes: &[u8]) -> Result<(usize, &[u8]), ParseError> {
    let (slice, rest) = match bytes.iter().position(|c| !(*c as char).is_numeric()) {
        Some(pos) => bytes.split_at(pos),
        None => (bytes, &[] as &[u8]),
    };

    match std::str::from_utf8(slice) {
        Ok(s) => match s.parse() {
            Ok(value) => Ok((value, rest)),
            Err(_) => Err(ParseError {
                line: slice,
                kind: ParseErrorKind::ParseError("line is not a valid proguard record"),
            }),
        },
        Err(err) => Err(ParseError {
            line: slice,
            kind: ParseErrorKind::Utf8Error(err),
        }),
    }
}

fn parse_prefix<'s>(bytes: &'s [u8], prefix: &'s [u8]) -> Result<&'s [u8], ParseError<'s>> {
    bytes.strip_prefix(prefix).ok_or(ParseError {
        line: bytes,
        kind: ParseErrorKind::ParseError("line is not a valid proguard record"),
    })
}

fn parse_until<'s, P>(bytes: &'s [u8], predicate: P) -> Result<(&'s str, &'s [u8]), ParseError<'s>>
where
    P: Fn(&u8) -> bool,
{
    let (slice, rest) = match bytes.iter().position(predicate) {
        Some(pos) => bytes.split_at(pos),
        None => (bytes, &[] as &[u8]),
    };

    match std::str::from_utf8(slice) {
        Ok(s) => Ok((s, rest)),
        Err(err) => Err(ParseError {
            line: slice,
            kind: ParseErrorKind::Utf8Error(err),
        }),
    }
}

fn parse_until_or_newline<'s, P>(
    bytes: &'s [u8],
    predicate: P,
) -> Result<(&'s str, &'s [u8]), ParseError<'s>>
where
    P: Fn(&u8) -> bool,
{
    match parse_until(bytes, |byte| is_newline(byte) || predicate(byte)) {
        Ok((slice, bytes)) => {
            if !bytes.is_empty() && is_newline(&bytes[0]) {
                Err(ParseError {
                    line: slice.as_bytes(),
                    kind: ParseErrorKind::ParseError("line is not a valid proguard record"),
                })
            } else {
                Ok((slice, bytes))
            }
        }
        Err(err) => Err(err),
    }
}

fn consume_leading_newlines(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|c| !is_newline(c)) {
        Some(pos) => &bytes[pos..],
        None => b"",
    }
}

fn split_line(bytes: &[u8]) -> (&[u8], &[u8]) {
    let pos = match bytes.iter().position(|c| is_newline(c)) {
        Some(pos) => pos + 1,
        None => bytes.len(),
    };

    bytes.split_at(pos)
}

fn is_newline(byte: &u8) -> bool {
    *byte == b'\r' || *byte == b'\n'
}
