//! Tests for R8 synthetic / lambda method retracing fixtures.
//!
//! These tests are based on the R8 retrace test suite from:
//! src/test/java/com/android/tools/r8/retrace/stacktraces/

use proguard::{ProguardCache, ProguardMapper, ProguardMapping};

// =============================================================================
// SyntheticLambdaMethodStackTrace
// =============================================================================

const SYNTHETIC_LAMBDA_METHOD_MAPPING: &str = "\
# {'id':'com.android.tools.r8.mapping','version':'1.0'}
example.Main -> example.Main:
    1:1:void main(java.lang.String[]):123 -> main
example.Foo -> a.a:
    5:5:void lambda$main$0():225 -> a
    3:3:void runIt():218 -> b
    2:2:void main():223 -> c
example.Foo$$ExternalSyntheticLambda0 -> a.b:
    void run(example.Foo) -> a
      # {'id':'com.android.tools.r8.synthesized'}
";

#[test]
fn test_synthetic_lambda_method_stacktrace() {
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
  at a.a.a(a.java:5)
  at a.b.a(Unknown Source)
  at a.a.b(a.java:3)
  at a.a.c(a.java:2)
  at example.Main.main(Main.java:1)
";

    // Note: this crate prints 4 spaces before each remapped frame.
    // Also, when no line info is available, it will output :0.
    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at example.Foo.lambda$main$0(Foo.java:225)
    at example.Foo.runIt(Foo.java:218)
    at example.Foo.main(Foo.java:223)
    at example.Main.main(Main.java:123)
";

    let mapper = ProguardMapper::from(SYNTHETIC_LAMBDA_METHOD_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(SYNTHETIC_LAMBDA_METHOD_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// SyntheticLambdaMethodWithInliningStackTrace
// =============================================================================

const SYNTHETIC_LAMBDA_METHOD_WITH_INLINING_MAPPING: &str = "\
# {'id':'com.android.tools.r8.mapping','version':'1.0'}
example.Main -> example.Main:
    1:1:void main(java.lang.String[]):123 -> main
example.Foo -> a.a:
    3:3:void runIt():218 -> b
    2:2:void main():223 -> c
example.Foo$$ExternalSyntheticLambda0 -> a.b:
    4:4:void example.Foo.lambda$main$0():225 -> a
    4:4:void run(example.Foo):0 -> a
      # {'id':'com.android.tools.r8.synthesized'}
";

#[test]
fn test_synthetic_lambda_method_with_inlining_stacktrace() {
    let input = "\
Exception in thread \"main\" java.lang.NullPointerException
  at a.b.a(Unknown Source:4)
  at a.a.b(a.java:3)
  at a.a.c(a.java:2)
  at example.Main.main(Main.java:1)
";

    let expected = "\
Exception in thread \"main\" java.lang.NullPointerException
    at example.Foo.lambda$main$0(Foo.java:225)
    at example.Foo.runIt(Foo.java:218)
    at example.Foo.main(Foo.java:223)
    at example.Main.main(Main.java:123)
";

    let mapper = ProguardMapper::from(SYNTHETIC_LAMBDA_METHOD_WITH_INLINING_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(SYNTHETIC_LAMBDA_METHOD_WITH_INLINING_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}

// =============================================================================
// MovedSynthetizedInfoStackTraceTest
// =============================================================================

const MOVED_SYNTHETIZED_INFO_MAPPING: &str = "\
# { id: 'com.android.tools.r8.mapping', version: '2.2' }
com.android.tools.r8.BaseCommand$Builder -> foo.bar:
    1:1:void inlinee(java.util.Collection):0:0 -> inlinee$synthetic
    1:1:void inlinee$synthetic(java.util.Collection):0:0 -> inlinee$synthetic
    2:2:void inlinee(java.util.Collection):206:206 -> inlinee$synthetic
    2:2:void inlinee$synthetic(java.util.Collection):0:0 -> inlinee$synthetic
      # {\"id\":\"com.android.tools.r8.synthesized\"}
    4:4:void inlinee(java.util.Collection):208:208 -> inlinee$synthetic
    4:4:void inlinee$synthetic(java.util.Collection):0 -> inlinee$synthetic
    7:7:void error(origin.Origin,java.lang.Throwable):363:363 -> inlinee$synthetic
    7:7:void inlinee(java.util.Collection):210 -> inlinee$synthetic
    7:7:void inlinee$synthetic(java.util.Collection):0:0 -> inlinee$synthetic
";

#[test]
fn test_moved_synthetized_info_stacktrace() {
    let input = "\
java.lang.RuntimeException: foobar
	at foo.bar.inlinee$synthetic(BaseCommand.java:2)
";

    let expected = "\
java.lang.RuntimeException: foobar
    at com.android.tools.r8.BaseCommand$Builder.inlinee(BaseCommand.java:206)
";

    let mapper = ProguardMapper::from(MOVED_SYNTHETIZED_INFO_MAPPING);
    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());

    let mapping = ProguardMapping::new(MOVED_SYNTHETIZED_INFO_MAPPING.as_bytes());
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual.trim(), expected.trim());
}
