//! This crate implements handling of proguard mapping files.
mod parser;

pub use parser::{Class, ClassIter, MappingView, MemberInfo, MemberIter, Parser};
