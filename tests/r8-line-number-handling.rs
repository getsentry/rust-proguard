//! Tests for R8 retrace "Line Number Handling" fixtures.
//!
//! Ported from the upstream R8 retrace fixtures in:
//! `src/test/java/com/android/tools/r8/retrace/stacktraces/`.
//!
//! Notes:
//! - These tests intentionally **omit** upstream `<OR>` markers and instead list alternative frames
//!   as duplicates, since this crate does not currently format `<OR>` groups.
//! - Fixture mapping indentation is normalized to 4-space member indentation so it is parsed by this
//!   crate's Proguard mapping parser.
//! - Expected stacktrace indentation is normalized to this crate's output (`"    at ..."`).
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
// NoObfuscationRangeMappingWithStackTrace
// =============================================================================

const NO_OBFUSCATION_RANGE_MAPPING_WITH_STACKTRACE_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> foo:
    void foo(long):1:1 -> a
    void bar(int):3 -> b
    void baz():0:0 -> c
    void main(java.lang.String[]):0 -> d
"#;

#[test]
fn test_no_obfuscation_range_mapping_with_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at foo.a(Bar.dummy:0)
	at foo.b(Foo.dummy:2)
	at foo.c(Baz.dummy:8)
	at foo.d(Qux.dummy:7)
"#;

    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.foo(Main.java:1)
    at com.android.tools.r8.naming.retrace.Main.bar(Main.java:3)
    at com.android.tools.r8.naming.retrace.Main.baz(Main.java)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java)
"#;

    assert_remap_stacktrace(
        NO_OBFUSCATION_RANGE_MAPPING_WITH_STACKTRACE_MAPPING,
        input,
        expected,
    );
}

// =============================================================================
// OutsideLineRangeStackTraceTest
// =============================================================================

const OUTSIDE_LINE_RANGE_STACKTRACE_MAPPING: &str = r#"some.other.Class -> a:
    void method1():42:42 -> a
some.Class -> b:
    1:3:void method2():11:13 -> a
    4:4:void method2():10:10 -> a
"#;

#[test]
fn test_outside_line_range_stacktrace() {
    let input = r#"java.io.IOException: INVALID_SENDER
    at a.a(:2)
    at a.a(Unknown Source)
    at b.a(:27)
    at b.a(Unknown Source)
"#;

    let expected = r#"java.io.IOException: INVALID_SENDER
    at some.other.Class.method1(Class.java:42)
    at some.other.Class.method1(Class.java:42)
    at some.Class.a(Class.java:27)
    at some.Class.method2(Class.java:0)
"#;

    assert_remap_stacktrace(OUTSIDE_LINE_RANGE_STACKTRACE_MAPPING, input, expected);
}

// =============================================================================
// PreambleLineNumberStackTrace
// =============================================================================

const PREAMBLE_LINE_NUMBER_MAPPING: &str = r#"kotlin.ResultKt -> kotlin.t:
    1:1:void createFailure(java.lang.Throwable):122:122 -> a
    2:2:void createFailure(java.lang.Throwable):124:124 -> a
"#;

#[test]
fn test_preamble_line_number_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
  at kotlin.t.a(SourceFile)
  at kotlin.t.a(SourceFile:0)
  at kotlin.t.a(SourceFile:1)
  at kotlin.t.a(SourceFile:2)
"#;

    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at kotlin.ResultKt.createFailure(Result.kt:0)
    at kotlin.ResultKt.createFailure(Result.kt:0)
    at kotlin.ResultKt.createFailure(Result.kt:122)
    at kotlin.ResultKt.createFailure(Result.kt:124)
"#;

    assert_remap_stacktrace(PREAMBLE_LINE_NUMBER_MAPPING, input, expected);
}

// =============================================================================
// DifferentLineNumberSpanStackTrace
// =============================================================================

const DIFFERENT_LINE_NUMBER_SPAN_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> a:
    void method1(java.lang.String):42:44 -> a
"#;

#[test]
fn test_different_line_number_span_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at a.a(Unknown Source:1)
"#;

    // Upstream emits 3 alternatives with `<OR>`. We list them as duplicates instead.
    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:42)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:43)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:44)
"#;

    assert_remap_stacktrace(DIFFERENT_LINE_NUMBER_SPAN_MAPPING, input, expected);
}

// =============================================================================
// ObfuscatedRangeToSingleLineStackTrace
// =============================================================================

const OBFUSCATED_RANGE_TO_SINGLE_LINE_MAPPING: &str = r#"foo.bar.Baz -> a:
    1:10:void qux():27:27 -> a
    11:15:void qux():42 -> a
    1337:1400:void foo.bar.Baz.quux():113:113 -> b
    1337:1400:void quuz():72 -> b
"#;

#[test]
fn test_obfuscated_range_to_single_line_stacktrace() {
    let input = r#"UnknownException: This is just a fake exception
  at a.a(:8)
  at a.a(:13)
  at a.b(:1399)
"#;

    let expected = r#"UnknownException: This is just a fake exception
    at foo.bar.Baz.qux(Baz.java:27)
    at foo.bar.Baz.qux(Baz.java:42)
    at foo.bar.Baz.quux(Baz.java:113)
    at foo.bar.Baz.quuz(Baz.java:72)
"#;

    assert_remap_stacktrace(OBFUSCATED_RANGE_TO_SINGLE_LINE_MAPPING, input, expected);
}

// =============================================================================
// NoObfuscatedLineNumberWithOverrideTest
// =============================================================================

const NO_OBFUSCATED_LINE_NUMBER_WITH_OVERRIDE_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> com.android.tools.r8.naming.retrace.Main:
    void main(java.lang.String):3 -> main
    void definedOverload():7 -> definedOverload
    void definedOverload(java.lang.String):11 -> definedOverload
    void overload1():7 -> overload
    void overload2(java.lang.String):11 -> overload
    void mainPC(java.lang.String[]):42 -> mainPC
"#;

#[test]
fn test_no_obfuscated_line_number_with_override() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(Unknown Source)
	at com.android.tools.r8.naming.retrace.Main.overload(Unknown Source)
	at com.android.tools.r8.naming.retrace.Main.definedOverload(Unknown Source)
	at com.android.tools.r8.naming.retrace.Main.mainPC(:3)
"#;

    // Upstream emits an `<OR>` alternative for `overload`. We list both as duplicates instead.
    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:3)
    at com.android.tools.r8.naming.retrace.Main.overload1(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.overload2(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.definedOverload(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.mainPC(Main.java:42)
"#;

    assert_remap_stacktrace(NO_OBFUSCATED_LINE_NUMBER_WITH_OVERRIDE_MAPPING, input, expected);
}

// =============================================================================
// SingleLineNoLineNumberStackTrace
// =============================================================================

const SINGLE_LINE_NO_LINE_NUMBER_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> foo.a:
    0:0:void method1(java.lang.String):42:42 -> a
    0:0:void main(java.lang.String[]):28 -> a
    0:0:void method2(java.lang.String):42:44 -> b
    0:0:void main2(java.lang.String[]):29 -> b
    void method3(java.lang.String):72:72 -> c
    void main3(java.lang.String[]):30 -> c
    void main4(java.lang.String[]):153 -> d
"#;

#[test]
fn test_single_line_no_line_number_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at foo.a.a(Unknown Source)
	at foo.a.b(Unknown Source)
	at foo.a.c(Unknown Source)
	at foo.a.d(Unknown Source)
"#;

    // Upstream emits an `<OR>` alternative for frame `c`. We list both as duplicates instead.
    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:42)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:28)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main2(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main3(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.method3(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main4(Main.java:153)
"#;

    assert_remap_stacktrace(SINGLE_LINE_NO_LINE_NUMBER_MAPPING, input, expected);
}

// =============================================================================
// MultipleLinesNoLineNumberStackTrace
// =============================================================================

const MULTIPLE_LINES_NO_LINE_NUMBER_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> foo.a:
    0:0:void method1(java.lang.String):42:42 -> a
    0:0:void main(java.lang.String[]):28 -> a
    1:1:void main(java.lang.String[]):153 -> a
"#;

#[test]
fn test_multiple_lines_no_line_number_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at foo.a.a(Unknown Source)
"#;

    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:42)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:28)
"#;

    assert_remap_stacktrace(MULTIPLE_LINES_NO_LINE_NUMBER_MAPPING, input, expected);
}

// =============================================================================
// InvalidMinifiedRangeStackTrace
// =============================================================================

const INVALID_MINIFIED_RANGE_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> com.android.tools.r8.naming.retrace.Main:
    5:3:void main(java.lang.String[]) -> main
"#;

#[test]
fn test_invalid_minified_range_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(Main.dummy:3)
"#;

    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:3)
"#;

    assert_remap_stacktrace(INVALID_MINIFIED_RANGE_MAPPING, input, expected);
}

// =============================================================================
// InvalidOriginalRangeStackTrace
// =============================================================================

const INVALID_ORIGINAL_RANGE_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> com.android.tools.r8.naming.retrace.Main:
    2:5:void main(java.lang.String[]):5:2 -> main
"#;

#[test]
fn test_invalid_original_range_stacktrace() {
    let input = r#"Exception in thread "main" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(Main.dummy:3)
"#;

    let expected = r#"Exception in thread "main" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:3)
"#;

    assert_remap_stacktrace(INVALID_ORIGINAL_RANGE_MAPPING, input, expected);
}


