//! This crate implements handling of proguard mapping files.
#![warn(missing_docs)]

mod parser;

pub use parser::{Class, ClassIter, MappingView, MemberInfo, MemberIter, Parser};
