//! Tests for R8 retrace "Method Overloading" fixtures.
//!
//! Ported from the upstream R8 retrace fixtures in:
//! - `src/test/java/com/android/tools/r8/retrace/stacktraces/OverloadedWithAndWithoutRangeStackTrace.java`
//! - `src/test/java/com/android/tools/r8/retrace/stacktraces/OverloadSameLineTest.java`
//! - `src/test/java/com/android/tools/r8/retrace/RetraceMappingWithOverloadsTest.java`
#![allow(clippy::unwrap_used)]

use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame};

fn assert_remap_stacktrace(mapping: &str, input: &str, expected: &str) {
    let mapper = ProguardMapper::from(mapping);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim_end(), expected.trim_end());

    let mapping = ProguardMapping::new(mapping.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim_end(), expected.trim_end());
}

// =============================================================================
// OverloadedWithAndWithoutRangeStackTrace
// =============================================================================

const OVERLOADED_WITH_AND_WITHOUT_RANGE_MAPPING: &str = r#"some.Class -> A:
    java.util.List select(java.util.List) -> a
    3:3:void sync():425:425 -> a
    void cancel(java.lang.String[]) -> a
"#;

#[test]
fn test_overloaded_with_and_without_range_stacktrace() {
    let input = r#"  at A.a(SourceFile:3)
"#;

    // This crate normalizes indentation to 4 spaces for parsed frames.
    let expected = r#"    at some.Class.sync(Class.java:425)
"#;

    assert_remap_stacktrace(OVERLOADED_WITH_AND_WITHOUT_RANGE_MAPPING, input, expected);
}

// =============================================================================
// OverloadSameLineTest
// =============================================================================

const OVERLOAD_SAME_LINE_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> foo.a:
    1:1:void overload():7:7 -> overload
    1:1:void overload(java.lang.String):13:13 -> overload
    1:1:void overload(int):15:15 -> overload
"#;

#[test]
fn test_overload_same_line_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at foo.a.overload(Main.java:1)
"#;

    // Upstream emits 3 alternatives (one per overload) for the same minified position.
    // This crate emits alternatives as duplicate frames (no `<OR>` markers).
    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.overload(Main.java:7)
    at com.android.tools.r8.naming.retrace.Main.overload(Main.java:13)
    at com.android.tools.r8.naming.retrace.Main.overload(Main.java:15)
"#;

    assert_remap_stacktrace(OVERLOAD_SAME_LINE_MAPPING, input, expected);
}

// =============================================================================
// RetraceMappingWithOverloadsTest (API-level behavior)
// =============================================================================

const RETRACE_MAPPING_WITH_OVERLOADS_MAPPING: &str = r#"some.Class -> A:
    java.util.List select(java.util.List) -> a
    3:3:void sync():425:425 -> a
    4:5:void sync():427:428 -> a
    void cancel(java.lang.String[]) -> a
"#;

#[test]
fn test_retrace_mapping_with_overloads_api_has_2_candidates_no_position() {
    let mapper = ProguardMapper::from(RETRACE_MAPPING_WITH_OVERLOADS_MAPPING);

    // For stacktrace remapping with no position (line == 0), align with R8's
    // "no position" semantics: do not include line-ranged mappings.
    let frame = StackFrame::new("A", "a", 0);
    let remapped: Vec<_> = mapper.remap_frame(&frame).collect();
    assert_eq!(remapped.len(), 2);

    let mapping = ProguardMapping::new(RETRACE_MAPPING_WITH_OVERLOADS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();
    let remapped: Vec<_> = cache.remap_frame(&frame).collect();
    assert_eq!(remapped.len(), 2);
}

#[test]
fn test_retrace_mapping_with_overloads_api_includes_sync_with_line() {
    let mapper = ProguardMapper::from(RETRACE_MAPPING_WITH_OVERLOADS_MAPPING);

    // When the minified line hits the `sync()` mapping range, it should produce a `sync` candidate.
    let frame = StackFrame::new("A", "a", 3);
    let remapped: Vec<_> = mapper.remap_frame(&frame).collect();
    assert!(remapped.iter().any(|f| f.method() == "sync"));

    let mapping = ProguardMapping::new(RETRACE_MAPPING_WITH_OVERLOADS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();
    let remapped: Vec<_> = cache.remap_frame(&frame).collect();
    assert!(remapped.iter().any(|f| f.method() == "sync"));
}
