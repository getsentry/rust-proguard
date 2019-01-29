use std::borrow::Cow;
use std::cmp::min;
use std::collections::HashMap;
use std::fmt;
use std::io::Result;
use std::io::{BufRead, BufReader};
use std::iter::Peekable;
use std::mem;
use std::path::Path;
use std::str;

use memmap::{Mmap, Protection};
use regex::bytes::{CaptureMatches, Regex};
use uuid::{Uuid, NAMESPACE_DNS};

lazy_static! {
static ref METHOD_RE: Regex = Regex::new(
    r#"(?m)^    (?:(\d+):(\d+):)?([^ ]+) ([^\(]+?)\(([^\)]*?)\)(:(\d+):(\d+))? -> ([\S]+)(?:\r?\n|$)"#).unwrap();
static ref CLASS_LINE_RE: Regex = Regex::new(
    r#"(?m)^([\S]+) -> ([\S]+?):(?:\r?\n|$)"#).unwrap();
static ref MEMBER_RE: Regex = Regex::new(
        r#"(?xm)^[\ ]{4}
            (?:(?P<start_ln>\d+):(?P<end_ln>\d+):)?
            (?P<type>[^\ ]+)\ 
            (?P<name>[^\(]+?)(?:\(
                (?P<args>[^\)]*?)\)(:
                (?P<lno_start_ln>\d+):
                (?P<lno_end_ln>\d+))?)?\ ->\ 
                (?P<alias>[\S]+)(\r?\n|$)"#).unwrap();
}

enum Backing<'a> {
    Buf(Cow<'a, [u8]>),
    Mmap(Mmap),
}

/// Represents class mapping information.
#[derive(Clone)]
pub struct Class<'a> {
    alias: &'a [u8],
    class_name: &'a [u8],
    buf: &'a [u8],
}

/// Represents a member of a class.
pub struct MemberInfo<'a> {
    alias: &'a [u8],
    ty: &'a [u8],
    name: &'a [u8],
    args: Option<Vec<&'a [u8]>>,
    lineno_range: Option<(u32, u32)>,
    // Available when Line Number Optimization (LNO) is used
    lno_lineno_range: Option<(u32, u32)>,
}

/// Represents arguments of a method.
pub struct Args<'a> {
    args: &'a [&'a [u8]],
    idx: usize,
}

/// Represents a view over a mapping text file.
pub struct MappingView<'a> {
    parser: Parser<'a>,
    header: Option<Header>,
    classes: HashMap<&'a str, Class<'a>>,
}

/// Parses a proguard file.
pub struct Parser<'a> {
    backing: Backing<'a>,
}

// The mapping header
// As added by the R8 compiler
#[derive(Debug, Default, Clone)]
pub struct Header {
    compiler: Option<String>,
    compiler_version: Option<String>,
    min_api: Option<String>,
}

impl Header {
    pub fn compiler(&self) -> Option<&str> {
        self.compiler.as_ref().map(|s| s.as_str())
    }
    pub fn compiler_version(&self) -> Option<&str> {
        self.compiler_version.as_ref().map(|s| s.as_str())
    }
    pub fn min_api(&self) -> Option<&str> {
        self.min_api.as_ref().map(|s| s.as_str())
    }
}

impl<'a> MappingView<'a> {
    fn from_parser(parser: Parser<'a>) -> Result<MappingView<'a>> {
        let mut view = MappingView {
            header: parser.parse_header()?,
            parser: parser,
            classes: HashMap::new(),
        };
        unsafe {
            let iter: ClassIter<'a> = mem::transmute(view.parser.classes());
            for class in iter {
                view.classes.insert(mem::transmute(class.alias()), class);
            }
        }
        Ok(view)
    }

    /// Creates a mapping view from a Cow buffer.
    pub fn from_cow(cow: Cow<'a, [u8]>) -> Result<MappingView<'a>> {
        MappingView::from_parser(Parser::from_cow(cow)?)
    }

    /// Creates a mapping from a borrowed byte slice.
    pub fn from_slice(buffer: &'a [u8]) -> Result<MappingView<'a>> {
        MappingView::from_cow(Cow::Borrowed(buffer))
    }

    /// Creates a mapping from an owned vector.
    pub fn from_vec(buffer: Vec<u8>) -> Result<MappingView<'a>> {
        MappingView::from_cow(Cow::Owned(buffer))
    }

    /// Opens a mapping view from a file on the file system.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<MappingView<'a>> {
        MappingView::from_parser(Parser::from_path(path)?)
    }

    /// Returns the UUID of the mapping file.
    pub fn uuid(&self) -> Uuid {
        self.parser.uuid()
    }

    /// Returns `true` if the mapping file contains line information.
    pub fn has_line_info(&self) -> bool {
        self.parser.has_line_info()
    }

    /// Locates a class by an obfuscated alias.
    pub fn find_class<'this>(&'this self, alias: &str) -> Option<&'this Class<'a>> {
        self.classes.get(alias)
    }

    pub fn header(&self) -> Option<&Header> {
        self.header.as_ref()
    }
}

impl<'a> Class<'a> {
    /// Returns the name of the class.
    pub fn class_name(&self) -> &str {
        str::from_utf8(self.class_name).unwrap_or("<unknown>")
    }

    /// Returns the obfuscated alias of a class.
    pub fn alias(&self) -> &str {
        str::from_utf8(self.alias).unwrap_or("<unknown>")
    }

    /// Looks up a field by an alias.
    pub fn get_field(&'a self, alias: &str) -> Option<MemberInfo<'a>> {
        self.members()
            .find(|x| !x.is_method() && x.alias() == alias)
    }

    /// Looks up all matching methods for a given alias.
    ///
    /// If the line number is supplied as well the return value will
    /// most likely only return a single item if found.
    pub fn get_methods(&'a self, alias: &str, lineno: Option<u32>) -> Vec<MemberInfo<'a>> {
        let mut rv: Vec<_> = self
            .members()
            .filter(|x| x.is_method() && x.alias() == alias && x.matches_line(lineno))
            .collect();
        rv.sort_by_key(|x| x.line_diff(lineno));
        rv
    }

    /// Iterates over all members of the class.
    pub fn members(&'_ self) -> MemberIter<'_> {
        let iter = MEMBER_RE.captures_iter(self.buf).peekable();
        MemberIter { iter: iter }
    }
}

impl<'a> fmt::Display for Class<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.class_name())
    }
}

impl<'a> fmt::Display for MemberInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.type_name(), self.name())?;
        if let Some(args) = self.args() {
            write!(f, "(")?;
            for (idx, arg) in args.enumerate() {
                if idx > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl<'a> Iterator for Args<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        loop {
            if self.idx >= self.args.len() {
                return None;
            }
            self.idx += 1;
            if let Ok(arg) = str::from_utf8(self.args[self.idx - 1]) {
                return Some(arg);
            }
        }
    }
}

/// Iterates over all classes.
pub struct ClassIter<'a> {
    buf: &'a [u8],
    iter: Peekable<CaptureMatches<'static, 'a>>,
}

/// Iterates over all members of a class.
pub struct MemberIter<'a> {
    iter: Peekable<CaptureMatches<'static, 'a>>,
}

impl<'a> Iterator for ClassIter<'a> {
    type Item = Class<'a>;

    fn next(&mut self) -> Option<Class<'a>> {
        if let Some(caps) = self.iter.next() {
            let class_name = caps.get(1).unwrap();
            let buf_start = caps.get(0).unwrap().end();
            let buf_end = if let Some(caps) = self.iter.peek() {
                caps.get(0).unwrap().start()
            } else {
                self.buf.len()
            };
            let alias_match = caps.get(2).unwrap();
            Some(Class {
                alias: alias_match.as_bytes(),
                class_name: class_name.as_bytes(),
                buf: &self.buf[buf_start..buf_end],
            })
        } else {
            None
        }
    }
}

impl<'a> Iterator for MemberIter<'a> {
    type Item = MemberInfo<'a>;

    fn next(&mut self) -> Option<MemberInfo<'a>> {
        if let Some(caps) = self.iter.next() {
            let src_from_line: u32 = caps
                .name("start_ln")
                .and_then(|x| str::from_utf8(x.as_bytes()).ok())
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            let dst_to_line: u32 = caps
                .name("end_ln")
                .and_then(|x| str::from_utf8(x.as_bytes()).ok())
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            let lno_src_from_line: u32 = caps
                .name("lno_start_ln")
                .and_then(|x| str::from_utf8(x.as_bytes()).ok())
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
            let lno_dst_to_line: u32 = caps
                .name("lno_end_ln")
                .and_then(|x| str::from_utf8(x.as_bytes()).ok())
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);

            Some(MemberInfo {
                alias: caps.name("alias").unwrap().as_bytes(),
                ty: caps.name("type").unwrap().as_bytes(),
                name: caps.name("name").unwrap().as_bytes(),
                args: caps.name("args").map(|x| {
                    x.as_bytes()
                        .split(|&x| x == b',')
                        .filter(|x| !x.is_empty())
                        .collect()
                }),
                lineno_range: if src_from_line > 0 && dst_to_line > 0 {
                    Some((src_from_line, dst_to_line))
                } else {
                    None
                },
                lno_lineno_range: if lno_src_from_line > 0 && lno_dst_to_line > 0 {
                    Some((lno_src_from_line, lno_dst_to_line))
                } else {
                    None
                },
            })
        } else {
            None
        }
    }
}

impl<'a> Parser<'a> {
    /// Creates a parser from a Cow buffer.
    pub fn from_cow(cow: Cow<'a, [u8]>) -> Result<Parser<'a>> {
        // parse header .. slice form the end of the header
        Ok(Parser {
            backing: Backing::Buf(cow),
        })
    }

    /// Creates a parser from a slice.
    pub fn from_slice(buffer: &'a [u8]) -> Result<Parser<'a>> {
        Parser::from_cow(Cow::Borrowed(buffer))
    }

    /// Creates a parser from a vec.
    pub fn from_vec(buffer: Vec<u8>) -> Result<Parser<'a>> {
        Parser::from_cow(Cow::Owned(buffer))
    }

    /// Creates a parser from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Parser<'a>> {
        let mmap = Mmap::open_path(path, Protection::Read)?;
        Ok(Parser {
            backing: Backing::Mmap(mmap),
        })
    }

    /// Calculates the UUID of the mapping file the parser looks at.
    pub fn uuid(&self) -> Uuid {
        let namespace = Uuid::new_v5(&NAMESPACE_DNS, "guardsquare.com");
        // this internally only operates on bytes, so this is safe to do
        Uuid::new_v5(&namespace, unsafe {
            str::from_utf8_unchecked(self.buffer())
        })
    }

    /// Returns `true` if the mapping file contains line information.
    pub fn has_line_info(&self) -> bool {
        let buf = self.buffer();
        for caps in METHOD_RE.captures_iter(buf) {
            if caps.get(1).is_some() {
                return true;
            }
        }

        false
    }

    /// Locates a class by an obfuscated alias.
    pub fn classes<'this>(&'this self) -> ClassIter<'this> {
        let buf = self.buffer();
        let iter = CLASS_LINE_RE.captures_iter(buf).peekable();
        ClassIter {
            buf: buf,
            iter: iter,
        }
    }

    #[inline(always)]
    fn buffer(&self) -> &[u8] {
        match self.backing {
            Backing::Buf(ref buf) => buf,
            Backing::Mmap(ref mmap) => unsafe { mmap.as_slice() },
        }
    }

    fn parse_header(&self) -> Result<Option<Header>> {
        let mut f = BufReader::new(self.buffer());
        let mut buf = String::new();

        let mut read = f.read_line(&mut buf)?;
        if !buf.starts_with('#') {
            return Ok(None);
        }

        let mut rv = Header::default();
        loop {
            if read == 0 || buf.trim_end().is_empty() {
                buf.truncate(buf.len() - read);
                break;
            }
            let mut iter = buf.splitn(2, ':');
            if let Some(key) = iter.next() {
                if let Some(value) = iter.next() {
                    let value = value.trim();
                    match key.to_lowercase().as_str() {
                        "# compiler" => rv.compiler = Some(value.to_string()),
                        "# compiler_version" => rv.compiler_version = Some(value.to_string()),
                        "# min_api" => rv.min_api = Some(value.to_string()),
                        _ => {}
                    }
                }
            }

            if !&buf.starts_with('#') {
                break;
            }

            buf.clear();
            read = f.read_line(&mut buf)?;
        }
        Ok(Some(rv))
    }
}

impl<'a> MemberInfo<'a> {
    /// Returns the alias of this member.
    pub fn alias(&self) -> &str {
        str::from_utf8(self.alias).unwrap_or("<unknown>")
    }

    /// Returns the type of this member or return value of method.
    pub fn type_name(&self) -> &str {
        str::from_utf8(self.ty).unwrap_or("<unknown>")
    }

    /// Returns the name of this member.
    pub fn name(&self) -> &str {
        str::from_utf8(self.name).unwrap_or("<unknown>")
    }

    /// Returns the args of this member if it's a method.
    pub fn args(&'a self) -> Option<Args<'a>> {
        self.args.as_ref().map(|args| Args {
            args: &args[..],
            idx: 0,
        })
    }

    /// Returns `true` if this is a method.
    pub fn is_method(&self) -> bool {
        self.args.is_some()
    }

    /// Returns the first line of this member range.
    pub fn first_line(&self) -> u32 {
        self.lno_lineno_range
            // has line number optimization
            .map(|x| x.0)
            .unwrap_or_else(|| self.lineno_range.map(|x| x.0).unwrap_or(0))
    }

    /// Returns the last line of this member range.
    pub fn last_line(&self) -> u32 {
        self.lno_lineno_range
            // has line number optimization
            .map(|x| x.1)
            .unwrap_or_else(|| self.lineno_range.map(|x| x.1).unwrap_or(0))
    }

    /// Returns the first line (optimized) of this member range if line number optimization was used.
    pub fn first_line_optimized(&self) -> Option<u32> {
        // If LNO was used, lineno_range holds the optimized value
        // where lno_lineno_range holds the actual line number
        if self.lno_lineno_range.is_some() {
            self.lineno_range.map(|x| x.0)
        } else {
            None
        }
    }

    /// Returns the last line (optimized) of this member range if line number optimization was used.
    pub fn last_line_optimized(&self) -> Option<u32> {
        // If LNO was used, lineno_range holds the optimized value
        // where lno_lineno_range holds the actual line number
        if self.lno_lineno_range.is_some() {
            self.lineno_range.map(|x| x.1)
        } else {
            None
        }
    }

    fn line_diff(&self, lineno: Option<u32>) -> u32 {
        (min(self.first_line() as i64, self.last_line() as i64) - (lineno.unwrap_or(0) as i64))
            .abs() as u32
    }

    fn matches_line(&self, lineno: Option<u32>) -> bool {
        let lineno = lineno.unwrap_or(0);
        if let Some((first, last)) = self.lineno_range {
            lineno == 0 || (first <= lineno && lineno <= last) || last == 0
        } else {
            true
        }
    }
}

#[test]
fn test_parse_header_complete() {
    let buf = br#"# compiler: R8
# compiler_version: 1.3.49
# min_api: 15
"#;

    let mapping = MappingView::from_slice(buf).expect("mapping");

    let parse_result = mapping.header();
    let header = parse_result.expect("header");
    assert_eq!(header.compiler().expect("compiler"), "R8");
    assert_eq!(header.compiler_version().expect("compiler"), "1.3.49");
    assert_eq!(header.min_api().expect("min_api"), "15");
}

#[test]
fn test_parse_header_compiler() {
    let buf = br#"# compiler: R8"#;

    let mapping = MappingView::from_slice(buf).expect("mapping");

    let parse_result = mapping.header();
    let header = parse_result.expect("header");
    assert_eq!(header.compiler().expect("compiler"), "R8");
    assert!(header.compiler_version().is_none());
    assert!(header.min_api().is_none());
}

#[test]
fn test_parse_header_compiler_version() {
    let buf = br#"# compiler_version: 1.3.49"#;

    let mapping = MappingView::from_slice(buf).expect("mapping");

    let parse_result = mapping.header();
    let header = parse_result.expect("header");
    assert!(header.compiler().is_none());
    assert_eq!(header.compiler_version().expect("compiler"), "1.3.49");
    assert!(header.min_api().is_none());
}

#[test]
fn test_parse_header_missing() {
    let buf = br#"android.arch.core.executor.ArchTaskExecutor -> a.a.a.a.c:"#;

    let mapping = MappingView::from_slice(buf).expect("mapping");

    let parse_result = mapping.header();
    assert!(parse_result.is_none());
}
