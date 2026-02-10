//! Internal helpers shared across modules.

/// For no-line mappings, prefer the original line if the mapping has no minified range.
pub(crate) fn resolve_no_line_output_line(
    frame_line: usize,
    original_startline: Option<usize>,
    startline: Option<usize>,
    endline: Option<usize>,
) -> usize {
    if frame_line > 0 {
        frame_line
    } else if startline.unwrap_or(0) == 0 && endline.unwrap_or(0) == 0 {
        original_startline.unwrap_or(0)
    } else {
        0
    }
}

pub(crate) fn extract_class_name(full_path: &str) -> Option<&str> {
    let after_last_period = full_path.split('.').next_back()?;
    // If the class is an inner class, we need to extract the outer class name
    after_last_period.split('$').next()
}

/// Synthesizes a source file name from a class name.
/// For Kotlin top-level classes ending in "Kt", the suffix is stripped and ".kt" is used.
/// Otherwise, the extension is derived from the reference file, defaulting to ".java".
/// For example: ("com.example.MainKt", Some("Other.java")) -> "Main.kt" (Kt suffix takes precedence)
/// For example: ("com.example.Main", Some("Other.kt")) -> "Main.kt"
/// For example: ("com.example.MainKt", None) -> "Main.kt"
/// For inner classes: ("com.example.Main$Inner", None) -> "Main.java"
pub(crate) fn synthesize_source_file(
    class_name: &str,
    reference_file: Option<&str>,
) -> Option<String> {
    let base = extract_class_name(class_name)?;

    // For Kotlin top-level classes (ending in "Kt"), always use .kt extension and strip suffix
    // This takes precedence over reference_file since Kt suffix is a strong Kotlin indicator
    if base.ends_with("Kt") && base.len() > 2 {
        let kotlin_base = &base[..base.len() - 2];
        return Some(format!("{}.kt", kotlin_base));
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
