//! Tests for R8 retrace "Exception Handling" fixtures.
//!
//! Ported from the upstream R8 retrace fixtures in:
//! `src/test/java/com/android/tools/r8/retrace/stacktraces/`.
//!
//! Notes:
//! - Fixture mapping indentation is normalized to 4-space member indentation so it is parsed by this
//!   crate's Proguard mapping parser.
//! - Expected stacktrace indentation is normalized to this crate's output (`"    at ..."`).
//! - These tests intentionally do **not** assert on R8 warning counts; this crate currently does not
//!   surface equivalent diagnostics.
#![allow(clippy::unwrap_used)]

use proguard::{ProguardCache, ProguardMapper, ProguardMapping};

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
// ObfucatedExceptionClassStackTrace
// =============================================================================

const OBFUSCATED_EXCEPTION_CLASS_MAPPING: &str = r#"foo.bar.baz -> a.b.c:
"#;

#[test]
fn test_obfuscated_exception_class_stacktrace() {
    let input = r#"a.b.c: Problem when compiling program
    at r8.main(App:800)
Caused by: a.b.c: You have to write the program first
    at r8.retrace(App:184)
    ... 7 more
"#;

    let expected = r#"foo.bar.baz: Problem when compiling program
    at r8.main(App:800)
Caused by: foo.bar.baz: You have to write the program first
    at r8.retrace(App:184)
    ... 7 more
"#;

    assert_remap_stacktrace(OBFUSCATED_EXCEPTION_CLASS_MAPPING, input, expected);
}

// =============================================================================
// SuppressedStackTrace
// =============================================================================

const SUPPRESSED_STACKTRACE_MAPPING: &str = r#"foo.bar.baz -> a.b.c:
"#;

#[test]
fn test_suppressed_stacktrace() {
    let input = r#"a.b.c: Problem when compiling program
    at r8.main(App:800)
Suppressed: a.b.c: You have to write the program first
    at r8.retrace(App:184)
    ... 7 more
"#;

    let expected = r#"foo.bar.baz: Problem when compiling program
    at r8.main(App:800)
Suppressed: foo.bar.baz: You have to write the program first
    at r8.retrace(App:184)
    ... 7 more
"#;

    assert_remap_stacktrace(SUPPRESSED_STACKTRACE_MAPPING, input, expected);
}

// =============================================================================
// CircularReferenceStackTrace
// =============================================================================

const CIRCULAR_REFERENCE_STACKTRACE_MAPPING: &str = r#"foo.bar.Baz -> A.A:
foo.bar.Qux -> A.B:
"#;

#[test]
fn test_circular_reference_stacktrace() {
    let input = r#"        [CIRCULAR REFERENCE: A.A]
 [CIRCULAR REFERENCE: A.B]
        [CIRCULAR REFERENCE: None.existing.class]
        [CIRCULAR REFERENCE: A.A] 
        [CIRCU:AA]
        [CIRCULAR REFERENCE: A.A
        [CIRCULAR REFERENCE: ]
        [CIRCULAR REFERENCE: None existing class]
"#;

    let expected = r#"        [CIRCULAR REFERENCE: foo.bar.Baz]
 [CIRCULAR REFERENCE: foo.bar.Qux]
        [CIRCULAR REFERENCE: None.existing.class]
        [CIRCULAR REFERENCE: foo.bar.Baz] 
        [CIRCU:AA]
        [CIRCULAR REFERENCE: foo.bar.Baz
        [CIRCULAR REFERENCE: ]
        [CIRCULAR REFERENCE: None existing class]
"#;

    assert_remap_stacktrace(CIRCULAR_REFERENCE_STACKTRACE_MAPPING, input, expected);
}

// =============================================================================
// ExceptionMessageWithClassNameInMessage
// =============================================================================

const EXCEPTION_MESSAGE_WITH_CLASSNAME_IN_MESSAGE_MAPPING: &str = r#"foo.bar.baz -> net:
"#;

#[test]
fn test_exception_message_with_class_name_in_message() {
    let input = r#"10-26 19:26:24.749 10159 26250 26363 E Tycho.crl: Exception
10-26 19:26:24.749 10159 26250 26363 E Tycho.crl: java.util.concurrent.ExecutionException: ary: eu: Exception in CronetUrlRequest: net::ERR_CONNECTION_CLOSED, ErrorCode=5, InternalErrorCode=-100, Retryable=true
"#;

    let expected = r#"10-26 19:26:24.749 10159 26250 26363 E Tycho.crl: Exception
10-26 19:26:24.749 10159 26250 26363 E Tycho.crl: java.util.concurrent.ExecutionException: ary: eu: Exception in CronetUrlRequest: foo.bar.baz::ERR_CONNECTION_CLOSED, ErrorCode=5, InternalErrorCode=-100, Retryable=true
"#;

    assert_remap_stacktrace(
        EXCEPTION_MESSAGE_WITH_CLASSNAME_IN_MESSAGE_MAPPING,
        input,
        expected,
    );
}

// =============================================================================
// RetraceAssertionErrorStackTrace
// =============================================================================

const RETRACE_ASSERTION_ERROR_STACKTRACE_MAPPING: &str = r#"com.android.tools.r8.retrace.Retrace -> com.android.tools.r8.retrace.Retrace:
    boolean $assertionsDisabled -> a
    1:5:void <clinit>():34:38 -> <clinit>
    1:1:void <init>():35:35 -> <init>
com.android.tools.r8.retrace.RetraceCore$StackTraceNode -> com.android.tools.r8.retrace.h:
    java.util.List lines -> a
    boolean $assertionsDisabled -> b
    1:1:void <clinit>():24:24 -> <clinit>
    1:4:void <init>(java.util.List):28:31 -> <init>
com.android.tools.r8.retrace.RetraceCore -> com.android.tools.r8.retrace.f:
    1:3:com.android.tools.r8.retrace.RetraceCore$RetraceResult retrace():106:108 -> a
    4:7:void retraceLine(java.util.List,int,java.util.List):112:115 -> a
    8:8:void retraceLine(java.util.List,int,java.util.List):115 -> a
    47:50:void retraceLine(java.util.List,int,java.util.List):116:119 -> a
com.android.tools.r8.retrace.Retrace -> com.android.tools.r8.retrace.Retrace:
    1:9:void run(com.android.tools.r8.retrace.RetraceCommand):112:120 -> run
"#;

#[test]
fn test_retrace_assertion_error_stacktrace() {
    let input = r#"java.lang.AssertionError
    at com.android.tools.r8.retrace.h.<init>(:4)
    at com.android.tools.r8.retrace.f.a(:48)
    at com.android.tools.r8.retrace.f.a(:2)
    at com.android.tools.r8.retrace.Retrace.run(:5)
    at com.android.tools.r8.retrace.RetraceTests.testNullLineTrace(RetraceTests.java:73)
"#;

    let expected = r#"java.lang.AssertionError
    at com.android.tools.r8.retrace.RetraceCore$StackTraceNode.<init>(RetraceCore.java:31)
    at com.android.tools.r8.retrace.RetraceCore.retraceLine(RetraceCore.java:117)
    at com.android.tools.r8.retrace.RetraceCore.retrace(RetraceCore.java:107)
    at com.android.tools.r8.retrace.Retrace.run(Retrace.java:116)
    at com.android.tools.r8.retrace.RetraceTests.testNullLineTrace(RetraceTests.java:73)
"#;

    assert_remap_stacktrace(RETRACE_ASSERTION_ERROR_STACKTRACE_MAPPING, input, expected);
}

// =============================================================================
// UnknownSourceStackTrace
// =============================================================================

const UNKNOWN_SOURCE_STACKTRACE_MAPPING: &str = r#"com.android.tools.r8.R8 -> a.a:
    void foo(int) -> a
    void bar(int, int) -> a
"#;

#[test]
fn test_unknown_source_stacktrace() {
    let input = r#"com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at a.a.a(Unknown Source)
    at a.a.a(Unknown Source)
    at com.android.tools.r8.R8.main(Unknown Source)
Caused by: com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at a.a.a(Unknown Source)
    ... 42 more
"#;

    // This crate does not format `<OR>` groups; alternatives are emitted as duplicate frames.
    let expected = r#"com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.foo(R8.java:0)
    at com.android.tools.r8.R8.main(Unknown Source)
Caused by: com.android.tools.r8.CompilationException: foo[parens](Source:3)
    at com.android.tools.r8.R8.bar(R8.java:0)
    at com.android.tools.r8.R8.foo(R8.java:0)
    ... 42 more
"#;

    assert_remap_stacktrace(UNKNOWN_SOURCE_STACKTRACE_MAPPING, input, expected);
}
