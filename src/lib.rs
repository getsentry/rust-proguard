//! This crate implements handling of proguard mapping files.

#![warn(missing_docs)]

mod mapper;
mod mapping;
mod stacktrace;

pub use mapper::Mapper;
pub use stacktrace::StackFrame;

// TODO: deprecate anything below

mod parser;
pub use parser::{Class, ClassIter, MappingView, MemberInfo, MemberIter, Parser};
