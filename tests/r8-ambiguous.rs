//! Tests for R8 ambiguous method retracing functionality.
//!
//! These tests are based on the R8 retrace test suite from:
//! src/test/java/com/android/tools/r8/retrace/stacktraces/

use proguard::{ProguardCache, ProguardMapper, ProguardMapping};

// =============================================================================
// AmbiguousStackTrace
// =============================================================================

const AMBIGUOUS_STACKTRACE_MAPPING: &str = "\
com.android.tools.r8.R8 -> a.a:
    void foo(int) -> a
    void bar(int, int) -> a
";

#[test]
fn test_ambiguous_stacktrace() {
    let input = "\
com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at a.a.a(Unknown Source)
    at a.a.a(Unknown Source)
    at com.android.tools.r8.R8.main(Unknown Source)
Caused by: com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at a.a.a(Unknown Source)
    ... 42 more
";

    let expected = "\
com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.main(Unknown Source)
Caused by: com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    ... 42 more
";

    let mapper = ProguardMapper::from(AMBIGUOUS_STACKTRACE_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_STACKTRACE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// AmbiguousMissingLineStackTrace
// =============================================================================

const AMBIGUOUS_MISSING_LINE_MAPPING: &str = "\
com.android.tools.r8.R8 -> a.a:
    void foo(int) -> a
    void bar(int, int) -> a
";

#[test]
fn test_ambiguous_missing_line_stacktrace() {
    let input = "\
com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at a.a.a(Unknown Source:7)
    at a.a.a(Unknown Source:8)
    at com.android.tools.r8.R8.main(Unknown Source)
Caused by: com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at a.a.a(Unknown Source:9)
    ... 42 more
";

    let expected = "\
com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.main(Unknown Source)
Caused by: com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    ... 42 more
";

    let mapper = ProguardMapper::from(AMBIGUOUS_MISSING_LINE_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_MISSING_LINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// AmbiguousInlineFramesStackTrace
// =============================================================================

const AMBIGUOUS_INLINE_FRAMES_MAPPING: &str = "\
com.android.tools.r8.R8 -> a.a:
    1:1:void foo(int):42:44 -> a
    1:1:void bar(int, int):32 -> a
    1:1:void baz(int, int):10 -> a
";

#[test]
fn test_ambiguous_inline_frames_stacktrace() {
    let input = "\
com.android.tools.r8.CompilationException:
    at a.a.a(Unknown Source:1)
";

    let expected = "\
com.android.tools.r8.CompilationException:
    at com.android.tools.r8.R8.foo(R8.java:42)
    at com.android.tools.r8.R8.bar(R8.java:32)
    at com.android.tools.r8.R8.baz(R8.java:10)
";

    let mapper = ProguardMapper::from(AMBIGUOUS_INLINE_FRAMES_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_INLINE_FRAMES_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// AmbiguousMultipleInlineStackTrace
// =============================================================================

const AMBIGUOUS_MULTIPLE_INLINE_MAPPING: &str = "\
com.android.tools.r8.Internal -> com.android.tools.r8.Internal:
    10:10:void some.inlinee1(int):10:10 -> zza
    10:10:void foo(int):10 -> zza
    11:12:void foo(int):11:12 -> zza
    10:10:void some.inlinee2(int, int):20:20 -> zza
    10:10:void foo(int, int):42 -> zza
";

#[test]
fn test_ambiguous_multiple_inline_stacktrace() {
    let input = "\
java.lang.IndexOutOfBoundsException
	at com.android.tools.r8.Internal.zza(SourceFile:10)
";

    let expected = "\
java.lang.IndexOutOfBoundsException
    at some.inlinee1(some.java:10)
    at com.android.tools.r8.Internal.foo(Internal.java:10)
    at some.inlinee2(some.java:20)
    at com.android.tools.r8.Internal.foo(Internal.java:42)
";

    let mapper = ProguardMapper::from(AMBIGUOUS_MULTIPLE_INLINE_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_MULTIPLE_INLINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// AmbiguousMethodVerboseStackTrace (non-verbose retrace output)
// =============================================================================

const AMBIGUOUS_METHOD_VERBOSE_MAPPING: &str = "\
com.android.tools.r8.naming.retrace.Main -> a.a:
    com.android.Foo main(java.lang.String[],com.android.Bar) -> a
    com.android.Foo main(java.lang.String[]) -> b
    void main(com.android.Bar) -> b
";

#[test]
fn test_ambiguous_method_verbose_stacktrace() {
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.c(Foo.java)
	at a.a.b(Bar.java)
	at a.a.a(Baz.java)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.c(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let mapper = ProguardMapper::from(AMBIGUOUS_METHOD_VERBOSE_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_METHOD_VERBOSE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// AmbiguousWithMultipleLineMappingsStackTrace
// =============================================================================

const AMBIGUOUS_WITH_MULTIPLE_LINE_MAPPINGS_MAPPING: &str = "\
com.android.tools.r8.Internal -> com.android.tools.r8.Internal:
    10:10:void foo(int):10:10 -> zza
    11:11:void foo(int):11:11 -> zza
    12:12:void foo(int):12:12 -> zza
";

#[test]
fn test_ambiguous_with_multiple_line_mappings_stacktrace() {
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

    let mapper = ProguardMapper::from(AMBIGUOUS_WITH_MULTIPLE_LINE_MAPPINGS_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_WITH_MULTIPLE_LINE_MAPPINGS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// AmbiguousWithSignatureStackTrace (non-verbose retrace output)
// =============================================================================

const AMBIGUOUS_WITH_SIGNATURE_MAPPING: &str = "\
com.android.tools.r8.Internal -> com.android.tools.r8.Internal:
    10:10:void foo(int):10:10 -> zza
    11:11:void foo(int, int):11:11 -> zza
    12:12:void foo(int, boolean):12:12 -> zza
    13:13:boolean foo(int, int):13:13 -> zza
";

#[test]
fn test_ambiguous_with_signature_stacktrace() {
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

    let mapper = ProguardMapper::from(AMBIGUOUS_WITH_SIGNATURE_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(AMBIGUOUS_WITH_SIGNATURE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlineNoLineAssumeNoInlineAmbiguousStackTrace
// =============================================================================

const INLINE_NO_LINE_ASSUME_NO_INLINE_AMBIGUOUS_MAPPING: &str = "\
retrace.Main -> a:
    void otherMain(java.lang.String[]) -> foo
    2:2:void method1(java.lang.String):0:0 -> foo
    2:2:void main(java.lang.String[]):0 -> foo
";

#[test]
fn test_inline_no_line_assume_no_inline_ambiguous_stacktrace() {
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.foo(Unknown Source)
";

    // When no line info is available, prefer base (no-line) mappings if present.
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at retrace.Main.otherMain(Main.java:0)
";

    let mapper = ProguardMapper::from(INLINE_NO_LINE_ASSUME_NO_INLINE_AMBIGUOUS_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping =
        ProguardMapping::new(INLINE_NO_LINE_ASSUME_NO_INLINE_AMBIGUOUS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}
