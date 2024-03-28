use crate::mapper::ProguardMapper;

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

fn byte_code_type_to_java_type(byte_code_type: &str, mapper: &ProguardMapper) -> String {
    let mut chrs = byte_code_type.chars();
    let token = chrs.next().unwrap_or_default();
    if token == 'L' {
        // invalid signature
        let l = chrs.clone().last();
        if l.is_none() || l.unwrap() != ';' {
            return "".to_string();
        }
        chrs.next_back(); // remove final `;`
        let obfuscated = chrs.collect::<String>().replace('/', ".");

        if let Some(mapped) = mapper.remap_class(&obfuscated) {
            return mapped.to_string();
        }

        return obfuscated;
    } else if token == '[' {
        let type_sig = chrs.clone().collect::<String>();
        if !type_sig.is_empty() {
            return format!(
                "{}[]",
                byte_code_type_to_java_type(chrs.collect::<String>().as_str(), mapper)
            );
        }
    } else if let Some(ty) = java_base_types(token) {
        return ty.to_string();
    }
    byte_code_type.to_string()
}

// parse_obfuscated_bytecode_signature will parse an obfuscated signatures into parameter
// and return types that can be then deobfuscated
fn parse_obfuscated_bytecode_signature(signature: &str) -> Option<(Vec<String>, String)> {
    let mut chrs = signature.chars();

    let token = chrs.next();
    if token.unwrap_or_default() != '(' {
        return None;
    }

    let sig = chrs.collect::<String>();
    let split_sign = sig.rsplitn(2, ')').collect::<Vec<&str>>();
    if split_sign.len() != 2 {
        return None;
    }

    let return_type = split_sign[0];
    let parameter_types = split_sign[1];
    if return_type.is_empty() {
        return None;
    }

    let mut types: Vec<String> = Vec::new();
    let mut tmp_buf: Vec<char> = Vec::new();

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
            types.push(tmp_buf.iter().collect());
            tmp_buf.clear();
        } else if token == '[' {
            tmp_buf.push('[');
        } else if let Some(ty) = java_base_types(token) {
            if !tmp_buf.is_empty() {
                tmp_buf.append(&mut ty.chars().collect::<Vec<char>>());
                types.push(tmp_buf.iter().collect());
                tmp_buf.clear();
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
        .map(|params| byte_code_type_to_java_type(params.as_str(), mapper))
        .collect();

    let return_java_type = if !return_type.is_empty() {
        byte_code_type_to_java_type(return_type.as_str(), mapper)
    } else {
        "".to_string()
    };

    Some((parameter_java_types, return_java_type))
}

/// formats types (param_type list, return_type) into a human-readable signature
pub fn format_signature(types: &Option<(Vec<String>, String)>) -> Option<String> {
    if types.is_none() {
        return None;
    }

    let (parameter_java_types, return_java_type) = types.as_ref().unwrap();

    let mut signature = format!("({})", parameter_java_types.join(", "));
    if !return_java_type.is_empty() && return_java_type != "void" {
        signature += format!(": {}", return_java_type).as_str();
    }

    Some(signature)
}

#[cfg(test)]
mod tests {
    use crate::{
        format_signature,
        java::{byte_code_type_to_java_type},
        ProguardMapper, ProguardMapping,
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
                byte_code_type_to_java_type(ty, &mapper),
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
            assert_eq!(format_signature(&signature), Some(expected.to_string()));
        }

        for obfuscated in tests_invalid {
            let signature = mapper.deobfuscate_signature(obfuscated);
            assert_eq!(format_signature(&signature), None);
        }
    }
}
