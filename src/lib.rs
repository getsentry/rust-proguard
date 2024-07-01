//! This crate implements handling of proguard mapping files.
//!
//! The main use case is to re-map classes or complete stack frames, but it can
//! also be used to parse a proguard mapping line-by-line.
//!
//! The `uuid` feature also allows getting the UUID of the proguard file.
//!
//! # Examples
//!
//! ```
//! let mapping = r#"
//! android.arch.core.internal.SafeIterableMap -> a.a.a.b.c:
//!     13:13:java.util.Map$Entry eldest():168:168 -> a
//! "#;
//! let mapper = proguard::ProguardMapper::from(mapping);
//!
//! // re-mapping a classname
//! assert_eq!(
//!     mapper.remap_class("a.a.a.b.c"),
//!     Some("android.arch.core.internal.SafeIterableMap"),
//! );
//!
//! // re-map a stack frame
//! assert_eq!(
//!     mapper
//!         .remap_frame(&proguard::StackFrame::new("a.a.a.b.c", "a", 13))
//!         .collect::<Vec<_>>(),
//!     vec![proguard::StackFrame::new(
//!         "android.arch.core.internal.SafeIterableMap",
//!         "eldest",
//!         168
//!     )],
//! );
//! ```

#![warn(missing_docs)]

mod cache;
mod java;
mod mapper;
mod mapping;
mod stacktrace;

pub use cache::{write_proguard_cache, Error, ErrorKind, IndexedProguard, ProguardCache};
pub use mapper::{DeobfuscatedSignature, ProguardMapper, RemappedFrameIter};
pub use mapping::{
    ClassIndex, LineMapping, MappingSummary, ParseError, ParseErrorKind, ProguardMapping,
    ProguardRecord, ProguardRecordIter,
};
pub use stacktrace::{StackFrame, StackTrace, Throwable};
