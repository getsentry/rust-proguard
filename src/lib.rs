//! This crate implements handling of proguard mapping files.
extern crate memmap;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate failure;
extern crate uuid;

mod parser;

pub use parser::{Class, ClassIter, MappingView, MemberInfo, MemberIter, Parser};
