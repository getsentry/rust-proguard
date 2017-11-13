//! This crate implements handling of proguard mapping files.
extern crate regex;
extern crate memmap;
#[macro_use] extern crate lazy_static;
extern crate uuid;

mod parser;

pub use parser::{MappingView, Class, ClassIter, MemberIter, MemberInfo, Parser};
