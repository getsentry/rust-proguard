//! Tests for R8 inline frame retracing functionality.
//!
//! These tests are based on the R8 retrace test suite from:
//! src/test/java/com/android/tools/r8/retrace/stacktraces/

use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame};

/// Test helper: simple remap_frame without rewrite rules or outline handling.
fn remap_frame_simple<'a>(
    cache: &'a ProguardCache<'a>,
    frame: &StackFrame<'a>,
) -> impl Iterator<Item = StackFrame<'a>> {
    let mut carried = None;
    cache
        .remap_frame(frame, None, false, &mut carried)
        .into_iter()
        .flatten()
}

// =============================================================================
// InlineWithLineNumbersStackTrace
// =============================================================================

const INLINE_WITH_LINE_NUMBERS_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> com.android.tools.r8.naming.retrace.Main:
    1:1:void main(java.lang.String[]):101:101 -> main
    2:4:void method1(java.lang.String):94:96 -> main
    2:4:void main(java.lang.String[]):102 -> main
    5:5:void method2(int):86:86 -> main
    5:5:void method1(java.lang.String):96 -> main
    5:5:void main(java.lang.String[]):102 -> main
    6:7:void method3(long):80:81 -> main
    6:7:void method2(int):88 -> main
    6:7:void method1(java.lang.String):96 -> main
    6:7:void main(java.lang.String[]):102 -> main
"#;

#[test]
fn test_inline_with_line_numbers() {
    let mapper = ProguardMapper::from(INLINE_WITH_LINE_NUMBERS_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(InliningRetraceTest.java:7)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method3(Main.java:81)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.java:88)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:96)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:102)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_with_line_numbers_cache() {
    let mapping = ProguardMapping::new(INLINE_WITH_LINE_NUMBERS_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(InliningRetraceTest.java:7)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method3(Main.java:81)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.java:88)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:96)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:102)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_with_line_numbers_frame() {
    let mapper = ProguardMapper::from(INLINE_WITH_LINE_NUMBERS_MAPPING);

    // Line 7 maps to the 6:7 range which has 4 inline frames
    let frames: Vec<_> = mapper
        .remap_frame(&StackFrame::new(
            "com.android.tools.r8.naming.retrace.Main",
            "main",
            7,
        ))
        .collect();

    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].method(), "method3");
    assert_eq!(frames[0].line(), 81);
    assert_eq!(frames[1].method(), "method2");
    assert_eq!(frames[1].line(), 88);
    assert_eq!(frames[2].method(), "method1");
    assert_eq!(frames[2].line(), 96);
    assert_eq!(frames[3].method(), "main");
    assert_eq!(frames[3].line(), 102);
}

// =============================================================================
// InlineNoLineNumberStackTrace
// =============================================================================

const INLINE_NO_LINE_NUMBER_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> com.android.tools.r8.naming.retrace.Main:
    1:1:void method1(java.lang.String):0:0 -> main
    1:1:void main(java.lang.String[]):0 -> main
    2:2:void method2(int):0:0 -> main
    2:2:void method1(java.lang.String):0 -> main
    2:2:void main(java.lang.String[]):0 -> main
    3:3:void method3(long):0:0 -> main
    3:3:void method2(int):0 -> main
    3:3:void method1(java.lang.String):0 -> main
    3:3:void main(java.lang.String[]):0 -> main
"#;

#[test]
fn test_inline_no_line_number() {
    let mapper = ProguardMapper::from(INLINE_NO_LINE_NUMBER_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(:3)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method3(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_no_line_number_cache() {
    let mapping = ProguardMapping::new(INLINE_NO_LINE_NUMBER_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(:3)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method3(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.java:0)
    at com.android.tools.r8.naming.retrace.Main.main(Main.java:0)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlineSourceFileStackTrace
// =============================================================================

const INLINE_SOURCE_FILE_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
com.android.tools.r8.naming.retrace.Main -> com.android.tools.r8.naming.retrace.Main:
# {"id":"sourceFile","fileName":"Main.kt"}
    1:1:void main(java.lang.String[]):101:101 -> main
    2:4:void method1(java.lang.String):94:96 -> main
    2:4:void main(java.lang.String[]):102 -> main
    5:5:void method2(int):86:86 -> main
    5:5:void method1(java.lang.String):96 -> main
    5:5:void main(java.lang.String[]):102 -> main
    6:7:void method3(long):80:81 -> main
    6:7:void method2(int):88 -> main
    6:7:void method1(java.lang.String):96 -> main
    6:7:void main(java.lang.String[]):102 -> main
"#;

#[test]
fn test_inline_source_file() {
    let mapper = ProguardMapper::from(INLINE_SOURCE_FILE_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(SourceFile:7)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method3(Main.kt:81)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.kt:88)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.kt:96)
    at com.android.tools.r8.naming.retrace.Main.main(Main.kt:102)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_source_file_cache() {
    let mapping = ProguardMapping::new(INLINE_SOURCE_FILE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at com.android.tools.r8.naming.retrace.Main.main(SourceFile:7)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.android.tools.r8.naming.retrace.Main.method3(Main.kt:81)
    at com.android.tools.r8.naming.retrace.Main.method2(Main.kt:88)
    at com.android.tools.r8.naming.retrace.Main.method1(Main.kt:96)
    at com.android.tools.r8.naming.retrace.Main.main(Main.kt:102)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlineFileNameStackTrace - Tests inline with different source files
// =============================================================================

const INLINE_FILE_NAME_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
com.android.tools.r8.naming.retrace.Main -> a.a:
# {"id":"sourceFile","fileName":"Main.kt"}
    1:1:void main(java.lang.String[]):101:101 -> a
    2:2:void foo.bar.Baz.inlinee():42:42 -> a
    2:2:void main(java.lang.String[]):102 -> a
"#;

#[test]
fn test_inline_file_name() {
    let mapper = ProguardMapper::from(INLINE_FILE_NAME_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:2)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at foo.bar.Baz.inlinee(Baz.kt:42)
    at com.android.tools.r8.naming.retrace.Main.main(Main.kt:102)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_file_name_cache() {
    let mapping = ProguardMapping::new(INLINE_FILE_NAME_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:2)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at foo.bar.Baz.inlinee(Baz.kt:42)
    at com.android.tools.r8.naming.retrace.Main.main(Main.kt:102)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlineFileNameWithInnerClassesStackTrace
// =============================================================================

const INLINE_FILE_NAME_INNER_CLASSES_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
com.android.tools.r8.naming.retrace.Main -> a.a:
# {"id":"sourceFile","fileName":"Main.kt"}
    1:1:void main(java.lang.String[]):101:101 -> a
    2:2:void foo.bar.Baz$Quux.inlinee():42:42 -> a
    2:2:void main(java.lang.String[]):102 -> a
"#;

#[test]
fn test_inline_file_name_with_inner_classes() {
    let mapper = ProguardMapper::from(INLINE_FILE_NAME_INNER_CLASSES_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:2)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at foo.bar.Baz$Quux.inlinee(Baz.kt:42)
    at com.android.tools.r8.naming.retrace.Main.main(Main.kt:102)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// NpeInlineRetraceStackTrace - Tests NPE with rewriteFrame rule
// =============================================================================

const NPE_INLINE_RETRACE_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
some.Class -> a:
    4:4:void other.Class():23:23 -> a
    4:4:void caller(other.Class):7 -> a
    # {"id":"com.android.tools.r8.rewriteFrame","conditions":["throws(Ljava/lang/NullPointerException;)"],"actions":["removeInnerFrames(1)"]}
"#;

#[test]
fn test_npe_inline_retrace() {
    let mapper = ProguardMapper::from(NPE_INLINE_RETRACE_MAPPING);

    let input = "\
java.lang.NullPointerException
	at a.a(:4)
";

    let expected = "\
java.lang.NullPointerException
    at some.Class.caller(Class.java:7)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_npe_inline_retrace_cache() {
    let mapping = ProguardMapping::new(NPE_INLINE_RETRACE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
java.lang.NullPointerException
	at a.a(:4)
";

    let expected = "\
java.lang.NullPointerException
    at some.Class.caller(Class.java:7)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlineRemoveFrameJava17StackTrace - Java 17 NPE with rewriteFrame
// =============================================================================

const INLINE_REMOVE_FRAME_JAVA17_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.2"}
foo.Class -> A:
    1:5:void inlinable():90:90 -> a
    1:5:void caller():97 -> a
      # {"id":"com.android.tools.r8.rewriteFrame","conditions":["throws(Ljava/lang/NullPointerException;)"],"actions":["removeInnerFrames(1)"]}
    1:5:void outerCaller():107 -> a
    1:1:void main():111:111 -> main
"#;

#[test]
fn test_inline_remove_frame_java17() {
    let mapper = ProguardMapper::from(INLINE_REMOVE_FRAME_JAVA17_MAPPING);

    let input = "\
java.lang.NullPointerException
	at A.a(SourceFile:1)
	at A.main(SourceFile:1)
";

    let expected = "\
java.lang.NullPointerException
    at foo.Class.caller(Class.java:97)
    at foo.Class.outerCaller(Class.java:107)
    at foo.Class.main(Class.java:111)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_remove_frame_java17_cache() {
    let mapping = ProguardMapping::new(INLINE_REMOVE_FRAME_JAVA17_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
java.lang.NullPointerException
    at A.a(SourceFile:1)
    at A.main(SourceFile:1)
";

    let expected = "\
java.lang.NullPointerException
    at foo.Class.caller(Class.java:97)
    at foo.Class.outerCaller(Class.java:107)
    at foo.Class.main(Class.java:111)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlineInOutlineStackTrace - Inline inside outline
// =============================================================================

const INLINE_IN_OUTLINE_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
outline.Class -> a:
    1:2:int outline():0 -> a
# {"id":"com.android.tools.r8.outline"}
some.Class -> b:
    1:1:void foo.bar.Baz.qux():42:42 -> s
    4:5:int foo.bar.baz.outlineCaller(int):98:99 -> s
    4:5:int outlineCaller(int):24 -> s
    27:27:int outlineCaller(int):0:0 -> s
# {"id":"com.android.tools.r8.outlineCallsite","positions":{"1":4,"2":5}}
"#;

#[test]
fn test_inline_in_outline() {
    let mapper = ProguardMapper::from(INLINE_IN_OUTLINE_MAPPING);

    let input = "\
java.io.IOException: INVALID_SENDER
	at a.a(:2)
	at b.s(:27)
";

    let expected = "\
java.io.IOException: INVALID_SENDER
    at foo.bar.baz.outlineCaller(baz.java:99)
    at some.Class.outlineCaller(Class.java:24)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_in_outline_cache() {
    let mapping = ProguardMapping::new(INLINE_IN_OUTLINE_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
java.io.IOException: INVALID_SENDER
	at a.a(:2)
	at b.s(:27)
";

    let expected = "\
java.io.IOException: INVALID_SENDER
    at foo.bar.baz.outlineCaller(baz.java:99)
    at some.Class.outlineCaller(Class.java:24)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// InlinePreambleNoOriginalStackTrace
// =============================================================================

const INLINE_PREAMBLE_NO_ORIGINAL_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
some.Class -> a:
    1:3:void caller():10 -> a
    4:5:void inlined():20:21 -> a
    4:5:void caller():11 -> a
"#;

#[test]
fn test_inline_preamble_no_original() {
    let mapper = ProguardMapper::from(INLINE_PREAMBLE_NO_ORIGINAL_MAPPING);

    // Test line 2 - should be in preamble range (1:3)
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 2)).collect();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].method(), "caller");
    assert_eq!(frames[0].line(), 10);

    // Test line 5 - should be in inline range (4:5)
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 5)).collect();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].method(), "inlined");
    assert_eq!(frames[0].line(), 21);
    assert_eq!(frames[1].method(), "caller");
    assert_eq!(frames[1].line(), 11);
}

// =============================================================================
// InlineSourceFileContextStackTrace - Different source files for inline frames
// =============================================================================

const INLINE_SOURCE_FILE_CONTEXT_MAPPING: &str = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
com.example.Main -> a.a:
# {"id":"sourceFile","fileName":"Main.kt"}
    1:1:void main(java.lang.String[]):10:10 -> a
    2:2:void com.example.util.Helper.doWork():50:50 -> a
    2:2:void main(java.lang.String[]):11 -> a
    3:3:void com.example.util.Helper.innerWork():60:60 -> a
    3:3:void com.example.util.Helper.doWork():51 -> a
    3:3:void main(java.lang.String[]):11 -> a
"#;

#[test]
fn test_inline_source_file_context() {
    let mapper = ProguardMapper::from(INLINE_SOURCE_FILE_CONTEXT_MAPPING);

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:3)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.example.util.Helper.innerWork(Helper.kt:60)
    at com.example.util.Helper.doWork(Helper.kt:51)
    at com.example.Main.main(Main.kt:11)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

#[test]
fn test_inline_source_file_context_cache() {
    let mapping = ProguardMapping::new(INLINE_SOURCE_FILE_CONTEXT_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
	at a.a.a(SourceFile:3)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at com.example.util.Helper.innerWork(Helper.kt:60)
    at com.example.util.Helper.doWork(Helper.kt:51)
    at com.example.Main.main(Main.kt:11)
";

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// Test individual frame remapping with different inline depths
// =============================================================================

#[test]
fn test_inline_frame_depth_one() {
    let mapping = r#"com.example.Main -> a:
    1:1:void foo():10:10 -> a
    2:2:void bar():20:20 -> a
    2:2:void foo():11 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    // Line 1 - no inlining
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 1)).collect();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].method(), "foo");
    assert_eq!(frames[0].line(), 10);

    // Line 2 - one level of inlining
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 2)).collect();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].method(), "bar");
    assert_eq!(frames[0].line(), 20);
    assert_eq!(frames[1].method(), "foo");
    assert_eq!(frames[1].line(), 11);
}

#[test]
fn test_inline_frame_depth_two() {
    let mapping = r#"com.example.Main -> a:
    1:1:void foo():10:10 -> a
    2:2:void baz():30:30 -> a
    2:2:void bar():21 -> a
    2:2:void foo():11 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    // Line 2 - two levels of inlining
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 2)).collect();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].method(), "baz");
    assert_eq!(frames[0].line(), 30);
    assert_eq!(frames[1].method(), "bar");
    assert_eq!(frames[1].line(), 21);
    assert_eq!(frames[2].method(), "foo");
    assert_eq!(frames[2].line(), 11);
}

#[test]
fn test_inline_frame_depth_two_cache() {
    let mapping = r#"com.example.Main -> a:
    1:1:void foo():10:10 -> a
    2:2:void baz():30:30 -> a
    2:2:void bar():21 -> a
    2:2:void foo():11 -> a
"#;
    let mapping = ProguardMapping::new(mapping.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    // Line 2 - two levels of inlining
    let frame = StackFrame::new("a", "a", 2);
    let frames: Vec<_> = remap_frame_simple(&cache, &frame).collect();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].method(), "baz");
    assert_eq!(frames[0].line(), 30);
    assert_eq!(frames[1].method(), "bar");
    assert_eq!(frames[1].line(), 21);
    assert_eq!(frames[2].method(), "foo");
    assert_eq!(frames[2].line(), 11);
}

// =============================================================================
// Test inline with line range spans
// =============================================================================

#[test]
fn test_inline_with_line_range() {
    let mapping = r#"com.example.Main -> a:
    1:5:void outer():10:14 -> a
    6:10:void inner():20:24 -> a
    6:10:void outer():15 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    // Line 3 - in outer range
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 3)).collect();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].method(), "outer");
    assert_eq!(frames[0].line(), 12); // 10 + (3-1) = 12

    // Line 8 - in inline range
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 8)).collect();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].method(), "inner");
    assert_eq!(frames[0].line(), 22); // 20 + (8-6) = 22
    assert_eq!(frames[1].method(), "outer");
    assert_eq!(frames[1].line(), 15);
}

// =============================================================================
// Test inline from different classes
// =============================================================================

#[test]
fn test_inline_from_different_class() {
    let mapping = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
com.example.Main -> a:
# {"id":"sourceFile","fileName":"Main.java"}
    1:1:void main():10:10 -> a
    2:2:void com.example.util.Utils.helper():50:50 -> a
    2:2:void main():11 -> a
    3:3:void com.example.lib.Library.work():100:100 -> a
    3:3:void com.example.util.Utils.helper():51 -> a
    3:3:void main():11 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 3)).collect();
    assert_eq!(frames.len(), 3);

    assert_eq!(frames[0].class(), "com.example.lib.Library");
    assert_eq!(frames[0].method(), "work");
    assert_eq!(frames[0].line(), 100);

    assert_eq!(frames[1].class(), "com.example.util.Utils");
    assert_eq!(frames[1].method(), "helper");
    assert_eq!(frames[1].line(), 51);

    assert_eq!(frames[2].class(), "com.example.Main");
    assert_eq!(frames[2].method(), "main");
    assert_eq!(frames[2].line(), 11);
}

#[test]
fn test_inline_from_different_class_stacktrace() {
    let mapping = r#"# {"id":"com.android.tools.r8.mapping","version":"2.0"}
com.example.Main -> a:
# {"id":"sourceFile","fileName":"Main.java"}
    1:1:void main():10:10 -> a
    2:2:void com.example.util.Utils.helper():50:50 -> a
    2:2:void main():11 -> a
    3:3:void com.example.lib.Library.work():100:100 -> a
    3:3:void com.example.util.Utils.helper():51 -> a
    3:3:void main():11 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    let input = "\
java.lang.RuntimeException: error
	at a.a(SourceFile:3)
";

    let expected = "\
java.lang.RuntimeException: error
    at com.example.lib.Library.work(Library.java:100)
    at com.example.util.Utils.helper(Utils.java:51)
    at com.example.Main.main(Main.java:11)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// Test edge case: no inline frames for non-matching line
// =============================================================================

#[test]
fn test_no_inline_for_non_matching_line() {
    let mapping = r#"com.example.Main -> a:
    1:5:void foo():10:14 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    // Line 10 is outside the mapped range (1-5), should return empty
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 10)).collect();
    assert_eq!(frames.len(), 0);
}

// =============================================================================
// InlineNoLineAssumeNoInlineAmbiguousStackTrace
// =============================================================================

const INLINE_NO_LINE_ASSUME_NO_INLINE_AMBIGUOUS_MAPPING: &str = r#"com.android.tools.r8.naming.retrace.Main -> a:
    void method1():0:0 -> a
    void method2():0:0 -> a
"#;

#[test]
fn test_inline_no_line_assume_no_inline_ambiguous() {
    let mapper = ProguardMapper::from(INLINE_NO_LINE_ASSUME_NO_INLINE_AMBIGUOUS_MAPPING);

    // When there's no line info and multiple methods map to the same name,
    // we get ambiguous results
    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 0)).collect();

    // Should return both possible methods (ambiguous)
    assert!(!frames.is_empty());
}

// =============================================================================
// Test inline with zero original line (placeholder)
// =============================================================================

#[test]
fn test_inline_with_zero_original_line() {
    let mapping = r#"com.example.Main -> a:
    1:1:void main():0:0 -> a
    1:1:void caller():10 -> a
"#;
    let mapper = ProguardMapper::from(mapping);

    let frames: Vec<_> = mapper.remap_frame(&StackFrame::new("a", "a", 1)).collect();
    // Should have 2 frames - the inline chain
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].method(), "main");
    assert_eq!(frames[0].line(), 0);
    assert_eq!(frames[1].method(), "caller");
    assert_eq!(frames[1].line(), 10);
}
