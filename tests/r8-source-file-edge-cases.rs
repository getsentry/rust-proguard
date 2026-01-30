//! Tests for R8 retrace "Source File Edge Cases" fixtures.
//!
//! These tests are ported from the upstream R8 retrace fixtures in:
//! `src/test/java/com/android/tools/r8/retrace/stacktraces/`.
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
// ColonInFileNameStackTrace
// =============================================================================

const COLON_IN_FILE_NAME_MAPPING: &str = "\
some.Class -> a:
# {\"id\":\"sourceFile\",\"fileName\":\"Class.kt\"}
    1:3:int strawberry(int):99:101 -> s
    4:5:int mango(float):121:122 -> s
    int passionFruit(float):121:121 -> t
";

#[test]
fn test_colon_in_file_name_stacktrace() {
    let input = r#"  at a.s(:foo::bar:1)
  at a.t(:foo::bar:)
"#;

    let expected = r#"    at some.Class.strawberry(Class.kt:99)
    at some.Class.passionFruit(Class.kt:121)
"#;

    assert_remap_stacktrace(COLON_IN_FILE_NAME_MAPPING, input, expected);
}

// =============================================================================
// UnicodeInFileNameStackTrace
// =============================================================================

const UNICODE_IN_FILE_NAME_MAPPING: &str = "\
some.Class -> a:
# {\"id\":\"sourceFile\",\"fileName\":\"Class.kt\"}
    1:3:int strawberry(int):99:101 -> s
    4:5:int mango(float):121:122 -> s
";

#[test]
fn test_unicode_in_file_name_stacktrace() {
    let input = r#"  at a.s(Blåbærgrød.jàvà:1)
"#;

    // Normalize indentation to this crate's output (`"    at ..."`)
    let expected = r#"    at some.Class.strawberry(Class.kt:99)
"#;

    assert_remap_stacktrace(UNICODE_IN_FILE_NAME_MAPPING, input, expected);
}

// =============================================================================
// MultipleDotsInFileNameStackTrace
// =============================================================================

const MULTIPLE_DOTS_IN_FILE_NAME_MAPPING: &str = "\
some.Class -> a:
# {\"id\":\"sourceFile\",\"fileName\":\"Class.kt\"}
    1:3:int strawberry(int):99:101 -> s
    4:5:int mango(float):121:122 -> s
";

#[test]
fn test_multiple_dots_in_file_name_stacktrace() {
    let input = r#"  at a.s(foo.bar.baz:1)
"#;

    let expected = r#"    at some.Class.strawberry(Class.kt:99)
"#;

    assert_remap_stacktrace(MULTIPLE_DOTS_IN_FILE_NAME_MAPPING, input, expected);
}

// =============================================================================
// SourceFileNameSynthesizeStackTrace
// =============================================================================

const SOURCE_FILE_NAME_SYNTHESIZE_MAPPING: &str = "\
android.support.v7.widget.ActionMenuView -> mapping:
    21:21:void invokeItem():624 -> a
android.support.v7.widget.ActionMenuViewKt -> mappingKotlin:
    21:21:void invokeItem():624 -> b
";

#[test]
fn test_source_file_name_synthesize_stacktrace() {
    let input = r#"    at mapping.a(AW779999992:21)
	at noMappingKt.noMapping(AW779999992:21)
	at mappingKotlin.b(AW779999992:21)
"#;

    let expected = r#"    at android.support.v7.widget.ActionMenuView.invokeItem(ActionMenuView.java:624)
	at noMappingKt.noMapping(AW779999992:21)
    at android.support.v7.widget.ActionMenuViewKt.invokeItem(ActionMenuView.kt:624)
"#;

    assert_remap_stacktrace(SOURCE_FILE_NAME_SYNTHESIZE_MAPPING, input, expected);
}

// =============================================================================
// SourceFileWithNumberAndEmptyStackTrace
// =============================================================================

const SOURCE_FILE_WITH_NUMBER_AND_EMPTY_MAPPING: &str = "\
com.android.tools.r8.R8 -> com.android.tools.r8.R8:
    34:34:void com.android.tools.r8.utils.ExceptionUtils.withR8CompilationHandler(com.android.tools.r8.utils.Reporter,com.android.tools.r8.utils.ExceptionUtils$CompileAction):59:59 -> a
    34:34:void runForTesting(com.android.tools.r8.utils.AndroidApp,com.android.tools.r8.utils.InternalOptions):261 -> a
";

#[test]
fn test_source_file_with_number_and_empty_stacktrace() {
    let input = r#"    at com.android.tools.r8.R8.a(R.java:34)
    at com.android.tools.r8.R8.a(:34)
"#;

    let expected = r#"    at com.android.tools.r8.utils.ExceptionUtils.withR8CompilationHandler(ExceptionUtils.java:59)
    at com.android.tools.r8.R8.runForTesting(R8.java:261)
    at com.android.tools.r8.utils.ExceptionUtils.withR8CompilationHandler(ExceptionUtils.java:59)
    at com.android.tools.r8.R8.runForTesting(R8.java:261)
"#;

    assert_remap_stacktrace(SOURCE_FILE_WITH_NUMBER_AND_EMPTY_MAPPING, input, expected);
}
