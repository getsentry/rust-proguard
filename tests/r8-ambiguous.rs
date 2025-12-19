//! Tests for ambiguous method retracing functionality.
//!
//! These tests are based on the R8 retrace test suite from:
//! src/test/java/com/android/tools/r8/retrace/stacktraces/
//!
//! When multiple original methods map to the same obfuscated name,
//! the retrace should return all possible alternatives.

use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame};

// =============================================================================
// AmbiguousStackTrace
// Multiple methods (foo and bar) map to the same obfuscated name 'a'
// =============================================================================

const AMBIGUOUS_MAPPING: &str = r#"com.android.tools.r8.R8 -> a.a:
    void foo(int) -> a
    void bar(int, int) -> a
"#;

#[test]
fn test_ambiguous_methods_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:0)
";

    // Both foo and bar should appear as alternatives
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_methods_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:0)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_methods_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MAPPING);

    let frame = StackFrame::new("a.a", "a", 0);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.R8", "foo", 0, "R8.java")
    );
    assert_eq!(
        frames[1],
        StackFrame::with_file("com.android.tools.r8.R8", "bar", 0, "R8.java")
    );
}

#[test]
fn test_ambiguous_methods_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("a.a", "a", 0);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.R8", "foo", 0, "R8.java")
    );
    assert_eq!(
        frames[1],
        StackFrame::with_file("com.android.tools.r8.R8", "bar", 0, "R8.java")
    );
}

// =============================================================================
// AmbiguousMissingLineStackTrace
// Ambiguous methods with line numbers that don't match any range
// =============================================================================

#[test]
fn test_ambiguous_with_line_number_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:7)
";

    // Line number 7 doesn't match any specific range, but methods have no line ranges
    // so both methods should still be returned (line preserved as 0)
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_with_line_number_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:7)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_with_line_number_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MAPPING);

    let frame = StackFrame::new("a.a", "a", 7);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 2);
}

#[test]
fn test_ambiguous_with_line_number_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("a.a", "a", 7);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 2);
}

// =============================================================================
// AmbiguousWithSignatureStackTrace
// Multiple overloaded methods with different signatures
// From R8: AmbiguousWithSignatureStackTrace.java
// =============================================================================

const AMBIGUOUS_SIGNATURE_MAPPING: &str = r#"com.android.tools.r8.Internal -> com.android.tools.r8.Internal:
    10:10:void foo(int):10:10 -> zza
    11:11:void foo(int, int):11:11 -> zza
    12:12:void foo(int, boolean):12:12 -> zza
    13:13:boolean foo(int, int):13:13 -> zza
"#;

#[test]
fn test_ambiguous_signature_no_line_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_SIGNATURE_MAPPING);

    // From R8: input has "Unknown" - no line number available
    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(Unknown)
";

    // R8 retrace shows all 4 overloads as ambiguous alternatives
    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_signature_no_line_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_SIGNATURE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(Unknown)
";

    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_signature_with_line_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_SIGNATURE_MAPPING);

    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    // Line 10 disambiguates to single method
    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_signature_with_line_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_SIGNATURE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_signature_with_line_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_SIGNATURE_MAPPING);

    let frame = StackFrame::new("com.android.tools.r8.Internal", "zza", 10);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.Internal", "foo", 10, "Internal.java")
    );
}

#[test]
fn test_ambiguous_signature_with_line_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_SIGNATURE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("com.android.tools.r8.Internal", "zza", 11);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.Internal", "foo", 11, "Internal.java")
    );
}

// =============================================================================
// AmbiguousWithMultipleLineMappingsStackTrace
// Same method with multiple line ranges
// From R8: AmbiguousWithMultipleLineMappingsStackTrace.java
// =============================================================================

const AMBIGUOUS_MULTIPLE_LINES_MAPPING: &str = r#"com.android.tools.r8.Internal -> com.android.tools.r8.Internal:
    10:10:void foo(int):10:10 -> zza
    11:11:void foo(int):11:11 -> zza
    12:12:void foo(int):12:12 -> zza
"#;

#[test]
fn test_ambiguous_multiple_lines_no_line_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MULTIPLE_LINES_MAPPING);

    // From R8: input has "Unknown" - no line number available
    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(Unknown)
";

    // All 3 map to same method signature, R8 shows one result with no line
    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_multiple_lines_no_line_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MULTIPLE_LINES_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(Unknown)
";

    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_multiple_lines_with_line_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MULTIPLE_LINES_MAPPING);

    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_multiple_lines_with_line_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MULTIPLE_LINES_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    let expected = "\
java.lang.IndexOutOfBoundsException
	at java.util.ArrayList.get(ArrayList.java:411)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_multiple_lines_with_line_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MULTIPLE_LINES_MAPPING);

    let frame = StackFrame::new("com.android.tools.r8.Internal", "zza", 10);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.Internal", "foo", 10, "Internal.java")
    );
}

#[test]
fn test_ambiguous_multiple_lines_with_line_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MULTIPLE_LINES_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("com.android.tools.r8.Internal", "zza", 11);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.Internal", "foo", 11, "Internal.java")
    );
}

// =============================================================================
// AmbiguousInlineFramesStackTrace
// Ambiguity in inline frame chain
// =============================================================================

const AMBIGUOUS_INLINE_MAPPING: &str = r#"com.android.tools.r8.R8 -> a.a:
    1:1:void foo(int):42:44 -> a
    1:1:void bar(int, int):32 -> a
    1:1:void baz(int, int):10 -> a
"#;

#[test]
fn test_ambiguous_inline_frames_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_INLINE_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:1)
";

    // Inline chain: foo (42-44) -> bar (32) -> baz (10)
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.R8.foo(R8.java:42)
    at com.android.tools.r8.R8.bar(R8.java:32)
    at com.android.tools.r8.R8.baz(R8.java:10)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_inline_frames_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_INLINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:1)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.R8.foo(R8.java:42)
    at com.android.tools.r8.R8.bar(R8.java:32)
    at com.android.tools.r8.R8.baz(R8.java:10)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_inline_frames_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_INLINE_MAPPING);

    let frame = StackFrame::new("a.a", "a", 1);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 3);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.R8", "foo", 42, "R8.java")
    );
    assert_eq!(
        frames[1],
        StackFrame::with_file("com.android.tools.r8.R8", "bar", 32, "R8.java")
    );
    assert_eq!(
        frames[2],
        StackFrame::with_file("com.android.tools.r8.R8", "baz", 10, "R8.java")
    );
}

#[test]
fn test_ambiguous_inline_frames_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_INLINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("a.a", "a", 1);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 3);
    assert_eq!(
        frames[0],
        StackFrame::with_file("com.android.tools.r8.R8", "foo", 42, "R8.java")
    );
    assert_eq!(
        frames[1],
        StackFrame::with_file("com.android.tools.r8.R8", "bar", 32, "R8.java")
    );
    assert_eq!(
        frames[2],
        StackFrame::with_file("com.android.tools.r8.R8", "baz", 10, "R8.java")
    );
}

// =============================================================================
// AmbiguousMultipleInlineStackTrace
// Multiple ambiguous inline frames from different classes
// =============================================================================

const AMBIGUOUS_MULTIPLE_INLINE_MAPPING: &str = r#"com.android.tools.r8.Internal -> com.android.tools.r8.Internal:
    10:10:void some.inlinee1(int):10:10 -> zza
    10:10:void foo(int):10 -> zza
    11:12:void foo(int):11:12 -> zza
    10:10:void some.inlinee2(int, int):20:20 -> zza
    10:10:void foo(int, int):42 -> zza
"#;

#[test]
fn test_ambiguous_multiple_inline_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MULTIPLE_INLINE_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    // Line 10 matches two inline chains:
    // Chain 1: inlinee1 -> foo(int)
    // Chain 2: inlinee2 -> foo(int, int)
    // Note: inlinee class is "some", so file synthesizes to "some.java"
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at some.inlinee1(some.java:10)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
    at some.inlinee2(some.java:20)
    at com.android.tools.r8.Internal.foo(Internal.java:42)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_multiple_inline_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MULTIPLE_INLINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at some.inlinee1(some.java:10)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
    at some.inlinee2(some.java:20)
    at com.android.tools.r8.Internal.foo(Internal.java:42)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_multiple_inline_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_MULTIPLE_INLINE_MAPPING);

    let frame = StackFrame::new("com.android.tools.r8.Internal", "zza", 10);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    // Two inline chains = 4 frames total
    assert_eq!(frames.len(), 4);
}

#[test]
fn test_ambiguous_multiple_inline_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_MULTIPLE_INLINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("com.android.tools.r8.Internal", "zza", 10);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 4);
}

// =============================================================================
// AmbiguousMethodVerboseStackTrace
// Different return types and parameters
// =============================================================================

const AMBIGUOUS_VERBOSE_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> a.a:
    com.android.Foo main(java.lang.String[],com.android.Bar) -> a
    com.android.Foo main(java.lang.String[]) -> b
    void main(com.android.Bar) -> b
"#;

#[test]
fn test_ambiguous_verbose_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_VERBOSE_MAPPING);

    // Method 'a' maps to single method
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:0)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_verbose_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_VERBOSE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:0)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_verbose_b_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_VERBOSE_MAPPING);

    // Method 'b' maps to two methods (ambiguous)
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.b(SourceFile:0)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_verbose_b_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_VERBOSE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.b(SourceFile:0)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_ambiguous_verbose_frame_mapper() {
    let mapper = ProguardMapper::from(AMBIGUOUS_VERBOSE_MAPPING);

    // Method 'a' maps to single method
    let frame_a = StackFrame::new("a.a", "a", 0);
    let frames_a: Vec<_> = mapper.remap_frame(&frame_a).collect();
    assert_eq!(frames_a.len(), 1);
    assert_eq!(
        frames_a[0],
        StackFrame::with_file(
            "com.android.tools.r8.naming.retrace.Main",
            "main",
            0,
            "Main.java"
        )
    );

    // Method 'b' maps to two methods
    let frame_b = StackFrame::new("a.a", "b", 0);
    let frames_b: Vec<_> = mapper.remap_frame(&frame_b).collect();
    assert_eq!(frames_b.len(), 2);
}

#[test]
fn test_ambiguous_verbose_frame_cache() {
    let mapping = ProguardMapping::new(AMBIGUOUS_VERBOSE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame_b = StackFrame::new("a.a", "b", 0);
    let frames_b: Vec<_> = cache.remap_frame(&frame_b).collect();
    assert_eq!(frames_b.len(), 2);
}

// =============================================================================
// InlineNoLineAssumeNoInlineAmbiguousStackTrace
// From R8: InlineNoLineAssumeNoInlineAmbiguousStackTrace.java
// Without line info, prefer non-inlined mapping over inlined
// =============================================================================

const INLINE_NO_LINE_MAPPING: &str = r#"retrace.Main -> a:
    void otherMain(java.lang.String[]) -> foo
    2:2:void method1(java.lang.String):0:0 -> foo
    2:2:void main(java.lang.String[]):0 -> foo
"#;

#[test]
fn test_inline_no_line_prefer_non_inline_mapper() {
    let mapper = ProguardMapper::from(INLINE_NO_LINE_MAPPING);

    // From R8: input has "Unknown Source" - no line number available
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.foo(Unknown Source)
";

    // R8 retrace prefers otherMain because it has no line range (not part of inline chain)
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at retrace.Main.otherMain(Main.java)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_no_line_prefer_non_inline_cache() {
    let mapping = ProguardMapping::new(INLINE_NO_LINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.foo(Unknown Source)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at retrace.Main.otherMain(Main.java)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_no_line_prefer_non_inline_frame_mapper() {
    let mapper = ProguardMapper::from(INLINE_NO_LINE_MAPPING);

    // Frame with line=0 should prefer non-inlined method
    let frame = StackFrame::new("a", "foo", 0);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0],
        StackFrame::with_file("retrace.Main", "otherMain", 0, "Main.java")
    );
}

#[test]
fn test_inline_no_line_prefer_non_inline_frame_cache() {
    let mapping = ProguardMapping::new(INLINE_NO_LINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("a", "foo", 0);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0],
        StackFrame::with_file("retrace.Main", "otherMain", 0, "Main.java")
    );
}

#[test]
fn test_inline_no_line_with_line_mapper() {
    let mapper = ProguardMapper::from(INLINE_NO_LINE_MAPPING);

    // With line 2, should match the inlined chain
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.foo(SourceFile:2)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at retrace.Main.method1(Main.java:0)
    at retrace.Main.main(Main.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_no_line_with_line_cache() {
    let mapping = ProguardMapping::new(INLINE_NO_LINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.foo(SourceFile:2)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at retrace.Main.method1(Main.java:0)
    at retrace.Main.main(Main.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_no_line_with_line_frame_mapper() {
    let mapper = ProguardMapper::from(INLINE_NO_LINE_MAPPING);

    let frame = StackFrame::new("a", "foo", 2);
    let frames: Vec<_> = mapper.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0],
        StackFrame::with_file("retrace.Main", "method1", 0, "Main.java")
    );
    assert_eq!(
        frames[1],
        StackFrame::with_file("retrace.Main", "main", 0, "Main.java")
    );
}

#[test]
fn test_inline_no_line_with_line_frame_cache() {
    let mapping = ProguardMapping::new(INLINE_NO_LINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();

    let frame = StackFrame::new("a", "foo", 2);
    let frames: Vec<_> = cache.remap_frame(&frame).collect();

    assert_eq!(frames.len(), 2);
    assert_eq!(
        frames[0],
        StackFrame::with_file("retrace.Main", "method1", 0, "Main.java")
    );
    assert_eq!(
        frames[1],
        StackFrame::with_file("retrace.Main", "main", 0, "Main.java")
    );
}
