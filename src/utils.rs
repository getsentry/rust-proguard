//! Internal helpers shared across modules.

pub(crate) fn extract_class_name(full_path: &str) -> Option<&str> {
    let after_last_period = full_path.split('.').next_back()?;
    // If the class is an inner class, we need to extract the outer class name
    after_last_period.split('$').next()
}

/// Synthesizes a source file name from a class name.
///
/// Any `$`-segment ending in `Kt` wins with a `.kt` extension — this covers
/// top-level Kotlin classes and Compose-compiler wrappers alike. Otherwise the
/// extension comes from `reference_file`, defaulting to `.java`.
///
/// See the tests at the bottom of this file for the full matrix of cases.
pub(crate) fn synthesize_source_file(
    class_name: &str,
    reference_file: Option<&str>,
) -> Option<String> {
    let last_segment = class_name.split('.').next_back()?;
    let mut segments = last_segment.split('$');
    let base = segments.next()?;

    // Kt suffix is a strong Kotlin indicator and takes precedence over reference_file.
    // Compiler-generated wrappers (e.g. `ComposableSingletons$MainActivityKt`) bury the
    // marker in an inner segment, so we scan every `$`-segment, not just `base`.
    for segment in std::iter::once(base).chain(segments) {
        if let Some(kotlin_base) = segment.strip_suffix("Kt").filter(|s| !s.is_empty()) {
            return Some(format!("{}.kt", kotlin_base));
        }
    }

    // If we have a reference file, derive extension from it
    if let Some(ext) = reference_file.and_then(|f| f.rfind('.').map(|pos| &f[pos..])) {
        return Some(format!("{}{}", base, ext));
    }

    Some(format!("{}.java", base))
}

/// Converts a Java class name to its JVM descriptor format.
///
/// For example, `java.lang.NullPointerException` becomes `Ljava/lang/NullPointerException;`.
pub fn class_name_to_descriptor(class: &str) -> String {
    let mut descriptor = String::with_capacity(class.len() + 2);
    descriptor.push('L');
    descriptor.push_str(&class.replace('.', "/"));
    descriptor.push(';');
    descriptor
}

#[cfg(test)]
mod tests {
    use super::synthesize_source_file;

    #[test]
    fn kotlin_top_level_class_uses_kt_extension() {
        assert_eq!(
            synthesize_source_file("com.example.MainKt", None).as_deref(),
            Some("Main.kt")
        );
    }

    #[test]
    fn kt_suffix_takes_precedence_over_reference_file() {
        assert_eq!(
            synthesize_source_file("com.example.MainKt", Some("Other.java")).as_deref(),
            Some("Main.kt")
        );
    }

    #[test]
    fn reference_file_extension_is_used_when_no_kt_suffix() {
        assert_eq!(
            synthesize_source_file("com.example.Main", Some("Other.kt")).as_deref(),
            Some("Main.kt")
        );
    }

    #[test]
    fn non_kotlin_inner_class_falls_back_to_outer_java() {
        assert_eq!(
            synthesize_source_file("com.example.Main$Inner", None).as_deref(),
            Some("Main.java")
        );
    }

    #[test]
    fn composable_singletons_wrapper_uses_inner_kt_segment() {
        assert_eq!(
            synthesize_source_file("com.example.ComposableSingletons$MainKt", None).as_deref(),
            Some("Main.kt")
        );
    }

    #[test]
    fn bare_kt_segment_is_not_a_kotlin_marker() {
        // Degenerate "Kt" alone (length 2) must not strip to an empty base.
        assert_eq!(
            synthesize_source_file("com.example.Foo$Kt", None).as_deref(),
            Some("Foo.java")
        );
    }

    #[test]
    fn default_extension_is_java() {
        assert_eq!(
            synthesize_source_file("com.example.Main", None).as_deref(),
            Some("Main.java")
        );
    }
}
