use std::fmt;
use std::str;
use std::cmp::min;
use std::path::Path;
use std::borrow::Cow;

use regex::bytes::Regex;
use memmap::{Mmap, Protection};
use prelude::*;


enum Backing<'a> {
    Buf(Cow<'a, [u8]>),
    Mmap(Mmap),
}

pub struct Class<'a> {
    alias: &'a [u8],
    class_name: &'a [u8],
    buf: &'a [u8],
}

pub struct FieldInfo<'a> {
    ty: &'a [u8],
    alias: &'a [u8],
    name: &'a [u8],
}

pub struct MethodInfo<'a> {
    alias: &'a [u8],
    return_value: &'a [u8],
    args: Vec<&'a [u8]>,
    method_name: &'a [u8],
    lineno_range: Option<(u32, u32)>,
}

pub struct Args<'a> {
    args: &'a[&'a [u8]],
    idx: usize,
}

pub struct MappingView<'a> {
    backing: Backing<'a>,
}

pub struct MappingInfo {
    has_line_info: bool,
}

impl MappingInfo {
    pub fn has_line_info(&self) -> bool {
        self.has_line_info
    }
}

impl<'a> MappingView<'a> {
    pub fn from_cow(cow: Cow<'a, [u8]>) -> Result<MappingView<'a>> {
        Ok(MappingView {
            backing: Backing::Buf(cow),
        })
    }

    pub fn from_slice(buffer: &'a [u8]) -> Result<MappingView<'a>> {
        MappingView::from_cow(Cow::Borrowed(buffer))
    }

    pub fn from_vec(buffer: Vec<u8>) -> Result<MappingView<'a>> {
        MappingView::from_cow(Cow::Owned(buffer))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<MappingView<'a>> {
        let mmap = Mmap::open_path(path, Protection::Read)?;
        Ok(MappingView {
            backing: Backing::Mmap(mmap),
        })
    }

    pub fn analyze_mapping(&self) -> MappingInfo {
        lazy_static! {
            static ref METHOD_RE: Regex = Regex::new(
                r#"(?m)^    (?:(\d+):(\d+):)?([\S]+) ([\S]+?)\(([\S]*?)\) -> ([\S]+)$"#).unwrap();
        }

        let mut rv = MappingInfo {
            has_line_info: false,
        };
        let buf = self.buffer();

        for caps in METHOD_RE.captures_iter(buf) {
            if caps.get(1).is_some() {
                rv.has_line_info = true;
                break;
            }
        }

        rv
    }

    pub fn find_class(&'a self, alias: &str) -> Option<Class<'a>> {
        lazy_static! {
            static ref CLASS_LINE_RE: Regex = Regex::new(
                r#"(?m)^([\S]+) -> ([\S]+?):(?:\r?\n|$)"#).unwrap();
        }
        let buf = self.buffer();
        let mut iter = CLASS_LINE_RE.captures_iter(buf);

        while let Some(caps) = iter.next() {
            if &caps[2] != alias.as_bytes() {
                continue;
            }

            let class_name = caps.get(1).unwrap();
            let buf_start = caps.get(0).unwrap().end();
            let buf_end = if let Some(caps) = iter.next() {
                caps.get(0).unwrap().start()
            } else {
                buf.len()
            };

            let alias_match = caps.get(2).unwrap();
            return Some(Class {
                alias: &buf[alias_match.start()..alias_match.end()],
                class_name: &buf[class_name.start()..class_name.end()],
                buf: &buf[buf_start..buf_end],
            });
        }

        None
    }

    #[inline(always)]
    fn buffer(&self) -> &[u8] {
        match self.backing {
            Backing::Buf(ref buf) => buf,
            Backing::Mmap(ref mmap) => unsafe { mmap.as_slice() }
        }
    }
}

impl<'a> Class<'a> {
    pub fn class_name(&self) -> &str {
        str::from_utf8(self.class_name).unwrap_or("<unknown>")
    }

    pub fn alias(&self) -> &str {
        str::from_utf8(self.alias).unwrap_or("<unknown>")
    }

    pub fn get_field(&'a self, alias: &str) -> Option<FieldInfo<'a>> {
        lazy_static! {
            static ref FIELD_RE: Regex = Regex::new(
                r#"(?m)^    ([\S]+) ([\S]+?) -> ([\S]+)$"#).unwrap();
        }
        let mut iter = FIELD_RE.captures_iter(self.buf);

        while let Some(caps) = iter.next() {
            let m_alias = caps.get(3).unwrap();
            if m_alias.as_bytes() == alias.as_bytes() {
                return Some(FieldInfo {
                    ty: caps.get(1).unwrap().as_bytes(),
                    name: caps.get(2).unwrap().as_bytes(),
                    alias: m_alias.as_bytes(),
                });
            }
        }

        None
    }

    pub fn get_methods(&'a self, alias: &str, lineno: Option<u32>)
        -> Vec<MethodInfo<'a>>
    {
        lazy_static! {
            static ref METHOD_RE: Regex = Regex::new(
                r#"(?m)^    (?:(\d+):(\d+):)?([\S]+) ([\S]+?)\(([\S]*?)\) -> ([\S]+)$"#).unwrap();
        }

        let mut rv = vec![];

        let mut iter = METHOD_RE.captures_iter(self.buf);

        while let Some(caps) = iter.next() {
            let m_alias = caps.get(6).unwrap();
            if m_alias.as_bytes() == alias.as_bytes() {
                let from_line: u32 = caps.get(1)
                    .and_then(|x| str::from_utf8(x.as_bytes()).ok())
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);
                let to_line: u32 = caps.get(2)
                    .and_then(|x| str::from_utf8(x.as_bytes()).ok())
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(0);

                let method = MethodInfo {
                    alias: m_alias.as_bytes(),
                    return_value: caps.get(3).unwrap().as_bytes(),
                    args: caps.get(5).unwrap().as_bytes().split(|&x| x == b',').collect(),
                    method_name: caps.get(4).unwrap().as_bytes(),
                    lineno_range: if from_line > 0 && to_line > 0 {
                        Some((from_line, to_line))
                    } else {
                        None
                    },
                };

                if method.matches_line(lineno) {
                    rv.push(method);
                }
            }
        }

        rv.sort_by_key(|x| x.line_diff(lineno));

        rv
    }
}

impl<'a> fmt::Display for Class<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.class_name())
    }
}

impl<'a> FieldInfo<'a> {

    pub fn name(&self) -> &str {
        str::from_utf8(self.name).unwrap_or("<invalid>")
    }

    pub fn alias(&self) -> &str {
        str::from_utf8(self.alias).unwrap_or("<invalid>")
    }

    pub fn type_name(&self) -> &str {
        str::from_utf8(self.ty).unwrap_or("<invalid>")
    }
}

impl<'a> fmt::Display for FieldInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.type_name(), self.name())
    }
}

impl<'a> MethodInfo<'a> {

    pub fn name(&self) -> &str {
        str::from_utf8(self.method_name).unwrap_or("<invalid>")
    }

    pub fn alias(&self) -> &str {
        str::from_utf8(self.alias).unwrap_or("<invalid>")
    }

    pub fn return_value(&self) -> &str {
        str::from_utf8(self.return_value).unwrap_or("<invalid>")
    }

    pub fn args(&'a self) -> Args<'a> {
        Args { args: &self.args[..], idx: 0 }
    }

    pub fn first_line(&self) -> u32 {
        self.lineno_range.map(|x| x.0).unwrap_or(0)
    }

    pub fn last_line(&self) -> u32 {
        self.lineno_range.map(|x| x.0).unwrap_or(0)
    }

    fn line_diff(&self, lineno: Option<u32>) -> u32 {
        (min(self.first_line() as i64, self.last_line() as i64) -
         (lineno.unwrap_or(0) as i64)).abs() as u32
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

impl<'a> fmt::Display for MethodInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}(", self.return_value(), self.name())?;
        for (idx, arg) in self.args().enumerate() {
            if idx > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", arg)?;
        }
        write!(f, ")")
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
