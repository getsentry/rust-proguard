//! A Parser for Proguard Mapping Files.
//!
//! The mapping file format is described
//! [here](https://www.guardsquare.com/en/products/proguard/manual/retrace).

#[cfg(feature = "uuid")]
use uuid_::Uuid;

/// A Proguard Mapping file.
#[derive(Clone, Default)]
pub struct ProguardMapping<'s> {
    source: &'s [u8],
}

impl<'s> std::fmt::Debug for ProguardMapping<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        let mut has_class_line = false;
        for record in self.iter().take(50) {
            match record {
                Ok(MappingRecord::Class { .. }) => {
                    has_class_line = true;
                }
                Ok(MappingRecord::Field { .. }) | Ok(MappingRecord::Method { .. })
                    if has_class_line =>
                {
                    return true;
                }
                _ => {}
            }
        }
        false
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
        for record in self.iter().take(100) {
            if let Ok(MappingRecord::Method { line_mapping, .. }) = record {
                return line_mapping.is_some();
            }
        }
        false
    }

    /// Calculates the UUID of the mapping file.
    ///
    /// The UUID is generated from a file checksum. See [more documentation] on
    /// how the UUID is generated.
    ///
    /// [more documentation]: https://docs.sentry.io/workflow/debug-files/#proguard-uuids
    #[cfg(feature = "uuid")]
    pub fn uuid(&self) -> Uuid {
        lazy_static::lazy_static! {
            static ref NAMESPACE: Uuid = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"guardsquare.com");
        }
        // this internally only operates on bytes, so this is safe to do
        Uuid::new_v5(&NAMESPACE, self.source)
    }

    /// Create an Iterator over [`MappingRecord`]s.
    ///
    /// [`MappingRecord`]: enum.MappingRecord.html
    pub fn iter(&self) -> MappingRecordIter<'s> {
        MappingRecordIter { slice: self.source }
    }
}

/// Split the input `slice` on line terminators.
///
/// This is basically [`str::lines`], except it works on a byte slice.
/// Also NOTE that it does not treat `\r\n` as a single line ending.
fn split_line(slice: &[u8]) -> (&[u8], &[u8]) {
    let pos = slice.iter().position(|c| *c == b'\n' || *c == b'\r');
    match pos {
        Some(pos) => (&slice[0..pos], &slice[pos + 1..]),
        None => (slice, &[]),
    }
}

/// An Iterator yielding [`MappingRecord`]s, created by [`ProguardMapping::iter`].
///
/// [`MappingRecord`]: enum.MappingRecord.html
/// [`ProguardMapping::iter`]: struct.ProguardMapping.html#method.iter
#[derive(Clone, Default)]
pub struct MappingRecordIter<'s> {
    slice: &'s [u8],
}

impl<'s> std::fmt::Debug for MappingRecordIter<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MappingRecordIter").finish()
    }
}

impl<'s> Iterator for MappingRecordIter<'s> {
    type Item = Result<MappingRecord<'s>, &'s [u8]>;
    fn next(&mut self) -> Option<Self::Item> {
        // We loop here, ignoring empty lines, which is important also because
        // `split_line` above would output an empty line for each `\r\n`.
        loop {
            let (line, rest) = split_line(self.slice);
            self.slice = rest;

            if !line.is_empty() {
                return Some(match MappingRecord::try_parse(line) {
                    Some(m) => Ok(m),
                    None => Err(line),
                });
            }
            if rest.is_empty() {
                return None;
            };
        }
    }
}

/// A proguard line mapping.
///
/// Maps start/end lines of a minified file to original start/end lines.
///
/// All line mappings are 1-based and inclusive.
#[derive(PartialEq, Default, Debug)]
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
#[derive(PartialEq, Debug)]
pub enum MappingRecord<'s> {
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

impl<'s> MappingRecord<'s> {
    /// Parses a line from a proguard mapping file.
    ///
    /// # Examples
    ///
    /// ```
    /// use proguard::MappingRecord;
    ///
    /// // Headers
    /// let parsed = MappingRecord::try_parse(b"# compiler: R8");
    /// assert_eq!(
    ///     parsed,
    ///     Some(MappingRecord::Header {
    ///         key: "compiler",
    ///         value: Some("R8")
    ///     })
    /// );
    ///
    /// // Class Mappings
    /// let parsed =
    ///     MappingRecord::try_parse(b"android.arch.core.executor.ArchTaskExecutor -> a.a.a.a.c:");
    /// assert_eq!(
    ///     parsed,
    ///     Some(MappingRecord::Class {
    ///         original: "android.arch.core.executor.ArchTaskExecutor",
    ///         obfuscated: "a.a.a.a.c"
    ///     })
    /// );
    ///
    /// // Field
    /// let parsed =
    ///     MappingRecord::try_parse(b"    android.arch.core.executor.ArchTaskExecutor sInstance -> a");
    /// assert_eq!(
    ///     parsed,
    ///     Some(MappingRecord::Field {
    ///         ty: "android.arch.core.executor.ArchTaskExecutor",
    ///         original: "sInstance",
    ///         obfuscated: "a",
    ///     })
    /// );
    ///
    /// // Method without line mappings
    /// let parsed = MappingRecord::try_parse(
    ///     b"    java.lang.Object putIfAbsent(java.lang.Object,java.lang.Object) -> b",
    /// );
    /// assert_eq!(
    ///     parsed,
    ///     Some(MappingRecord::Method {
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
    /// let parsed = MappingRecord::try_parse(
    ///     b"    1016:1016:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked",
    /// );
    /// assert_eq!(
    ///     parsed,
    ///     Some(MappingRecord::Method {
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
    pub fn try_parse(line: &'s [u8]) -> Option<Self> {
        let line = std::str::from_utf8(line).ok()?;
        parse_mapping(line)
    }
}

/// Parses a single line from a Proguard File.
///
/// Returns `None` if the line could not be parsed.
fn parse_mapping(mut line: &str) -> Option<MappingRecord> {
    if line.starts_with('#') {
        let mut split = line[1..].splitn(2, ':');
        let key = split.next()?.trim();
        let value = split.next().map(|s| s.trim());
        return Some(MappingRecord::Header { key, value });
    }
    if !line.starts_with("    ") {
        // class line: `originalclassname -> obfuscatedclassname:`
        let mut split = line.splitn(3, ' ');
        let original = split.next()?;
        if split.next()? != "->" || !line.ends_with(':') {
            return None;
        }
        let mut obfuscated = split.next()?;
        obfuscated = &obfuscated[..obfuscated.len() - 1];
        return Some(MappingRecord::Class {
            original,
            obfuscated,
        });
    }
    // field line or method line:
    // `originalfieldtype originalfieldname -> obfuscatedfieldname`
    // `[startline:endline:]originalreturntype [originalclassname.]originalmethodname(originalargumenttype,...)[:originalstartline[:originalendline]] -> obfuscatedmethodname`
    line = &line[4..];
    let mut line_mapping = LineMapping::default();

    // leading line mapping
    if line.starts_with(char::is_numeric) {
        let mut nums = line.splitn(3, ':');
        line_mapping.startline = nums.next()?.parse().ok()?;
        line_mapping.endline = nums.next()?.parse().ok()?;
        line = nums.next()?;
    }

    // split the type, name and obfuscated name
    let mut split = line.splitn(4, ' ');
    let ty = split.next()?;
    let mut original = split.next()?;
    if split.next()? != "->" {
        return None;
    }
    let obfuscated = split.next()?;

    // split off trailing line mappings
    let mut nums = original.splitn(3, ':');
    original = nums.next()?;
    line_mapping.original_startline = match nums.next() {
        Some(n) => Some(n.parse().ok()?),
        _ => None,
    };
    line_mapping.original_endline = match nums.next() {
        Some(n) => Some(n.parse().ok()?),
        _ => None,
    };

    // split off the arguments
    let mut args = original.splitn(2, '(');
    original = args.next()?;

    Some(match args.next() {
        None => MappingRecord::Field {
            ty,
            original,
            obfuscated,
        },
        Some(args) => {
            if !args.ends_with(')') {
                return None;
            }
            let arguments = &args[..args.len() - 1];

            let mut split_class = original.rsplitn(2, '.');
            original = split_class.next()?;
            let original_class = split_class.next();

            MappingRecord::Method {
                ty,
                original,
                obfuscated,
                arguments,
                original_class,
                line_mapping: if line_mapping.startline > 0 {
                    Some(line_mapping)
                } else {
                    None
                },
            }
        }
    })
}
