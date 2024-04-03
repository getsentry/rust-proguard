use std::ops::Index;

use crate::mapper::{DeobfuscatedSignature, ProguardMapper};

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
    let mut chrs = byte_code_type.char_indices();
    //let (idx, token) = chrs.next()?;
    let mut suffix = String::new();
    while let Some((idx, token)) = chrs.next() {
        if token == 'L' {
            // expect and remove final `;`
            if chrs.next_back()?.1 != ';' {
                return None;
            }
            let obfuscated = byte_code_type
                .index(idx + 1..byte_code_type.len() - 1)
                .replace('/', ".");

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

// parse_obfuscated_bytecode_signature will parse an obfuscated signatures into parameter
// and return types that can be then deobfuscated
fn parse_obfuscated_bytecode_signature(signature: &str) -> Option<(Vec<String>, String)> {
    let signature = signature.strip_prefix('(')?;

    let (parameter_types, return_type) = signature.rsplit_once(')')?;
    if return_type.is_empty() {
        return None;
    }

    let mut types: Vec<String> = Vec::new();
    let mut tmp_buf: String = String::new();

    let mut param_chrs = parameter_types.chars();
    while let Some(token) = param_chrs.next() {
        if token == 'L' {
            tmp_buf.push(token);
            for c in param_chrs.by_ref() {
                tmp_buf.push(c);
                if c == ';' {
                    break;
                }
            }
            if tmp_buf.is_empty() || !tmp_buf.ends_with(&[';']) {
                return None;
            }
            types.push(tmp_buf);
            tmp_buf = String::new();
        } else if token == '[' {
            tmp_buf.push('[');
        } else if java_base_types(token).is_some() {
            if !tmp_buf.is_empty() {
                tmp_buf.push(token);
                types.push(tmp_buf);
                tmp_buf = String::new();
            } else {
                types.push(token.to_string());
            }
        } else {
            tmp_buf.clear();
        }
    }

    Some((types, return_type.to_string()))
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
        .filter_map(|params| byte_code_type_to_java_type(params.as_str(), mapper))
        .collect();

    let return_java_type = byte_code_type_to_java_type(return_type.as_str(), mapper)?;

    Some((parameter_java_types, return_java_type))
}

/// formats types (param_type list, return_type) into a human-readable signature
pub fn format_signature(types: Option<DeobfuscatedSignature>) -> Option<String> {
    let types = types?;

    let parameter_java_types = types.parameters_types();
    let return_java_type = types.return_type();

    let mut signature = format!("({})", parameter_java_types.collect::<Vec<_>>().join(", "));
    if !return_java_type.is_empty() && return_java_type != "void" {
        signature.push_str(": ");
        signature.push_str(return_java_type);
    }

    Some(signature)
}

#[cfg(test)]
mod tests {
    use crate::{
        format_signature, java::byte_code_type_to_java_type, ProguardMapper, ProguardMapping,
    };
    use std::collections::HashMap;

    #[test]
    fn test_byte_code_type_to_java_type() {
        let proguard_source = b"org.slf4j.helpers.Util$ClassContextSecurityManager -> org.a.b.g$a:
    65:65:void <init>() -> <init>";

        let mapping = ProguardMapping::new(proguard_source);
        let mapper = ProguardMapper::new(mapping);

        let tests = HashMap::from([
            // invalid types
            ("", ""),
            ("L", ""),
            ("", ""),
            // valid types
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

        for (ty, expected) in tests {
            assert_eq!(
                byte_code_type_to_java_type(ty, &mapper).unwrap_or_default(),
                expected.to_string()
            );
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
            assert_eq!(format_signature(signature), Some(expected.to_string()));
        }

        for obfuscated in tests_invalid {
            let signature = mapper.deobfuscate_signature(obfuscated);
            assert_eq!(format_signature(signature), None);
        }
    }
}
