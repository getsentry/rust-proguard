use crate::{mapper::ProguardMapper, ProguardCache};

fn java_base_types(encoded_ty: char) -> Option<&'static str> {
    match encoded_ty {
        'Z' => Some("boolean"),
        'B' => Some("byte"),
        'C' => Some("char"),
        'S' => Some("short"),
        'I' => Some("int"),
        'J' => Some("long"),
        'F' => Some("float"),
        'D' => Some("double"),
        'V' => Some("void"),
        _ => None,
    }
}

fn byte_code_type_to_java_type(byte_code_type: &str, mapper: &ProguardMapper) -> Option<String> {
    let mut chrs = byte_code_type.chars();
    let mut suffix = "".to_string();
    while let Some(token) = chrs.next() {
        if token == 'L' {
            // expect and remove final `;`
            if chrs.next_back()? != ';' {
                return None;
            }
            let obfuscated = chrs.as_str().replace('/', ".");

            if let Some(mapped) = mapper.remap_class(&obfuscated) {
                return Some(format!("{}{}", mapped, suffix));
            }

            return Some(format!("{}{}", obfuscated, suffix));
        } else if token == '[' {
            suffix.push_str("[]");
            continue;
        } else if let Some(ty) = java_base_types(token) {
            return Some(format!("{}{}", ty, suffix));
        }
    }
    None
}

/// Same as [`byte_code_type_to_java_type`], but uses a [`ProguardCache`] for remapping.
fn byte_code_type_to_java_type_cache(
    byte_code_type: &str,
    cache: &ProguardCache,
) -> Option<String> {
    let mut chrs = byte_code_type.chars();
    let mut suffix = "".to_string();
    while let Some(token) = chrs.next() {
        if token == 'L' {
            // expect and remove final `;`
            if chrs.next_back()? != ';' {
                return None;
            }
            let obfuscated = chrs.as_str().replace('/', ".");

            if let Some(mapped) = cache.remap_class(&obfuscated) {
                return Some(format!("{}{}", mapped, suffix));
            }

            return Some(format!("{}{}", obfuscated, suffix));
        } else if token == '[' {
            suffix.push_str("[]");
            continue;
        } else if let Some(ty) = java_base_types(token) {
            return Some(format!("{}{}", ty, suffix));
        }
    }
    None
}

// parse_obfuscated_bytecode_signature will parse an obfuscated signatures into parameter
// and return types that can be then deobfuscated
fn parse_obfuscated_bytecode_signature(signature: &str) -> Option<(Vec<&str>, &str)> {
    let signature = signature.strip_prefix('(')?;

    let (parameter_types, return_type) = signature.rsplit_once(')')?;
    if return_type.is_empty() {
        return None;
    }

    let mut types: Vec<&str> = Vec::new();
    let mut first_idx = 0;

    let mut param_chrs = parameter_types.char_indices();
    while let Some((idx, token)) = param_chrs.next() {
        if token == 'L' {
            let mut last_idx = idx;
            for (i, c) in param_chrs.by_ref() {
                last_idx = i;
                if c == ';' {
                    break;
                }
            }
            let ty = parameter_types.get(first_idx..last_idx + 1)?;
            if ty.is_empty() || !ty.ends_with([';']) {
                return None;
            }
            types.push(ty);
            first_idx = last_idx + 1;
        } else if token == '[' {
            continue;
        } else if java_base_types(token).is_some() {
            let ty = parameter_types.get(first_idx..idx + 1)?;
            types.push(ty);
            first_idx = idx + 1;
        }
    }

    Some((types, return_type))
}

/// returns a tuple where the first element is the list of the function
/// parameters and the second one is the return type
pub fn deobfuscate_bytecode_signature(
    signature: &str,
    mapper: &ProguardMapper,
) -> Option<(Vec<String>, String)> {
    let (parameter_types, return_type) = parse_obfuscated_bytecode_signature(signature)?;
    let parameter_java_types: Vec<String> = parameter_types
        .into_iter()
        .filter(|params| !params.is_empty())
        .filter_map(|params| byte_code_type_to_java_type(params, mapper))
        .collect();

    let return_java_type = byte_code_type_to_java_type(return_type, mapper)?;

    Some((parameter_java_types, return_java_type))
}

/// Same as [`deobfuscate_bytecode_signature`], but uses a [`ProguardCache`] for remapping.
pub fn deobfuscate_bytecode_signature_cache(
    signature: &str,
    cache: &ProguardCache,
) -> Option<(Vec<String>, String)> {
    let (parameter_types, return_type) = parse_obfuscated_bytecode_signature(signature)?;
    let parameter_java_types: Vec<String> = parameter_types
        .into_iter()
        .filter(|params| !params.is_empty())
        .filter_map(|params| byte_code_type_to_java_type_cache(params, cache))
        .collect();

    let return_java_type = byte_code_type_to_java_type_cache(return_type, cache)?;

    Some((parameter_java_types, return_java_type))
}

#[cfg(test)]
mod tests {
    use crate::{java::byte_code_type_to_java_type, ProguardMapper, ProguardMapping};
    use std::collections::HashMap;

    #[test]
    fn test_byte_code_type_to_java_type() {
        let proguard_source = b"org.slf4j.helpers.Util$ClassContextSecurityManager -> org.a.b.g$a:
    65:65:void <init>() -> <init>";

        let mapping = ProguardMapping::new(proguard_source);
        let mapper = ProguardMapper::new(mapping);

        let tests = HashMap::from([
            ("[I", "int[]"),
            ("I", "int"),
            ("[Ljava/lang/String;", "java.lang.String[]"),
            ("[[J", "long[][]"),
            ("[B", "byte[]"),
            (
                // Obfuscated class type
                "Lorg/a/b/g$a;",
                "org.slf4j.helpers.Util$ClassContextSecurityManager",
            ),
        ]);

        // invalid types
        let tests_invalid = vec!["", "L", ""];

        for (ty, expected) in tests {
            assert_eq!(
                byte_code_type_to_java_type(ty, &mapper).unwrap(),
                expected.to_string()
            );
        }

        for ty in tests_invalid {
            let java_type = byte_code_type_to_java_type(ty, &mapper);
            assert!(java_type.is_none());
        }
    }

    #[test]
    fn test_format_signature() {
        let proguard_source = b"org.slf4j.helpers.Util$ClassContextSecurityManager -> org.a.b.g$a:
    65:65:void <init>() -> <init>";

        let mapping = ProguardMapping::new(proguard_source);
        let mapper = ProguardMapper::new(mapping);

        let tests_valid = HashMap::from([
            // valid signatures
            ("()V", "()"),
            ("([I)V", "(int[])"),
            ("(III)V", "(int, int, int)"),
            ("([Ljava/lang/String;)V", "(java.lang.String[])"),
            ("([[J)V", "(long[][])"),
            ("(I)I", "(int): int"),
            ("([B)V", "(byte[])"),
            (
                "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                "(java.lang.String, java.lang.String): java.lang.String",
            ),
            (
                // Obfuscated class type
                "(Lorg/a/b/g$a;)V",
                "(org.slf4j.helpers.Util$ClassContextSecurityManager)",
            ),
        ]);

        // invalid signatures
        let tests_invalid = vec!["", "()", "(L)"];

        for (obfuscated, expected) in tests_valid {
            let signature = mapper.deobfuscate_signature(obfuscated);
            assert_eq!(signature.unwrap().format_signature(), expected.to_string());
        }

        for obfuscated in tests_invalid {
            let signature = mapper.deobfuscate_signature(obfuscated);
            assert!(signature.is_none());
        }
    }
}
