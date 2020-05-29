//! This crate implements handling of proguard mapping files.
//!
//! The main use case is to re-map classes or complete stack frames, but it can
//! also be used to parse a proguard mapping line-by-line.
//!
//! # Examples
//!
//! ```
//! let mapping = br#"android.arch.core.internal.SafeIterableMap -> a.a.a.b.c:
//!     13:13:java.util.Map$Entry eldest():168:168 -> a"#;
//! let mapper = proguard::Mapper::new(mapping);
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

mod mapper;
mod mapping;
mod stacktrace;

pub use mapper::Mapper;
pub use mapping::{LineMapping, MappingRecord};
pub use stacktrace::StackFrame;

#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Calculates the UUID of the mapping file.
#[cfg(feature = "uuid")]
pub fn mapping_uuid(mapping: &[u8]) -> Uuid {
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"guardsquare.com");
    // this internally only operates on bytes, so this is safe to do
    Uuid::new_v5(&namespace, mapping)
}
