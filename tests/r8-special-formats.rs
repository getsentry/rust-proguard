//! Tests for R8 retrace "Special Formats" fixtures.
//!
//! Ported from the upstream R8 retrace fixtures in:
//! - `src/test/java/com/android/tools/r8/retrace/stacktraces/NamedModuleStackTrace.java`
//! - `src/test/java/com/android/tools/r8/retrace/stacktraces/AutoStackTrace.java`
//! - `src/test/java/com/android/tools/r8/retrace/stacktraces/PGStackTrace.java`
//! - `src/test/java/com/android/tools/r8/retrace/stacktraces/LongLineStackTrace.java`
//!
//! Notes:
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
// AutoStackTrace
// =============================================================================

const AUTO_STACKTRACE_MAPPING: &str = r#"com.android.tools.r8.AutoTest -> qtr:
    46:46:void foo(int):200:200 -> a
    17:19:void foo(int,int):23:25 -> a
"#;

#[test]
fn test_auto_stacktrace() {
    let input = r#"java.io.IOException: INVALID_SENDER
	at qtr.a(:com.google.android.gms@203915081@20.39.15 (060808-335085812):46)
	at qtr.a(:com.google.android.gms@203915081@20.39.15 (060808-335085812):18)
"#;

    let expected = r#"java.io.IOException: INVALID_SENDER
    at com.android.tools.r8.AutoTest.foo(AutoTest.java:200)
    at com.android.tools.r8.AutoTest.foo(AutoTest.java:24)
"#;

    assert_remap_stacktrace(AUTO_STACKTRACE_MAPPING, input, expected);
}

// =============================================================================
// LongLineStackTrace
// =============================================================================

#[test]
fn test_long_line_stacktrace_passthrough() {
    // Upstream fixture has an empty mapping and expects the single long line to be preserved.
    // We keep this as a smoke test that we don't crash on very long lines.
    let input = r#"asdf():::asfasidfsadfsafassdfsalfkaskldfasjkl908435 439593409 5309 843 5980349085 9043598 04930 5 9084389 549 385 908435098435980 4390 5890435908 4389 0509345890 23904239s909090safasiofas90f0-safads0-fas0-f-0f-0fasdf0-asswioj df jaiowj fioweoji fqiwope fopiwqej fqweiopj fwqejiof qwoepijf eiwoj fqwioepjf wiqeof jqweoifiqu t8324981qu2398 rt3289 rt2489t euhiorjg kdfgf8u432iojt3u8io432jk t3u49t 489u4389u t438u9 t43y89t 3 489t y8934t34 89ytu8943tu8 984u3 t 8u934asdf(:asdfas0dfasd0fa0)S)DFD)SDF_SD)FSDKFJlsalk;dfjaklsdf())(SDFSdfaklsdfas0d9fwe89rio223oi4rwoiuqaoiwqiowpjaklcvewtujoiwrjweof asdfjaswdj foisadj f aswioj df jaiowj fioweoji fqiwope fopiwqej fqweiopj fwqejiof qwoepijf eiwoj fqwioepjf wiqeof jqweoifiqu t8324981qu2398 rt3289 rt2489t euhiorjg kdfgf8u432iojt3u8io432jk t3u49t 489u4389u t438u9 t43y89t 3 489t y8934t34 89ytu8943tu8 984u3 t 8u93asdf(:asdfas0dfasd0fa0)S)DFD)SDF_SD)FSDKFJlsalk;dfjaklsdf())(SDFSdfaklsdfas0d9fwe89rio223oi4rwoiuqaoiwqiowpjaklcvewtujoiwrjweof asdfjaswdj foisadj f aswioj df jaiowj fioweoji fqiwope fopiwqej fqweiopj fwqejiof qwoepijf eiwoj fqwioepjf wiqeof jqweoifiqu t8324981qu2398 rt3289 rt2489t euhiorjg kdfgf8u432iojt3u8io432jk t3u49t 489u4389u t438u9 t43y89t 3 489t y8934t34 89ytu8943tu8 984u3 t 8u9344asdf(:asdfas0dfasd0fa0)S)DFD)SDF_SD)FSDKFJlsalk;dfjaklsdf())(SDFSdfaklsdfas0d9fwe89rio223oi4rwoiuqaoiwqiowpjaklcvewtujoiwrjweof asdfjaswdj foisadj f aswioj df jaiowj fioweoji fqiwope fopiwqej fqweiopj fwqejiof qwoepijf eiwoj fqwioepjf wiqeof jqweoifiqu t8324981qu2398 rt3289 rt2489t euhiorjg kdfgfasdfsdf123asdfas9dfas09df9a090asdasdfasfasdf290909009fw90wf9w0f9e09w0f90w
"#;

    let expected = input;
    assert_remap_stacktrace("", input, expected);
}
