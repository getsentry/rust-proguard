//! A Parser for Proguard Mapping Files.
//!
//! The mapping file format is described
//! [here](https://www.guardsquare.com/en/products/proguard/manual/retrace).

/// A proguard line mapping.
#[derive(PartialEq, Default, Debug)]
pub struct LineMapping {
    pub startline: usize,
    pub endline: usize,
    pub original_startline: Option<usize>,
    pub original_endline: Option<usize>,
}

/// A Proguard Mapping Record.
#[derive(PartialEq, Debug)]
pub enum MappingRecord<'s> {
    /// A Proguard Header KV pair.
    Header {
        key: &'s str,
        value: Option<&'s str>,
    },
    /// A Class line.
    ///
    /// `originalclassname -> obfuscatedclassname:`
    Class {
        original: &'s str,
        obfuscated: &'s str,
    },
    /// A Field or Method line.
    ///
    /// * `originalfieldtype originalfieldname -> obfuscatedfieldname`
    /// * `[startline:endline:]originalreturntype [originalclassname.]originalmethodname(originalargumenttype,...)[:originalstartline[:originalendline]] -> obfuscatedmethodname`
    Member {
        ty: &'s str,
        original: &'s str,
        obfuscated: &'s str,
        original_class: Option<&'s str>,
        arguments: Option<&'s str>,
        line_mapping: Option<LineMapping>,
    },
}

/// Parses a single line from a Proguard File.
///
/// Returns [`None`] if the line could not be parsed.
pub fn parse_mapping_line(mut line: &str) -> Option<MappingRecord> {
    if line.starts_with('#') {
        let mut split = line[1..].splitn(2, ':');
        let key = split.next()?.trim();
        let value = split.next().map(|s| s.trim());
        return Some(MappingRecord::Header { key, value });
    }
    if !line.starts_with("    ") {
        // class line: `originalclassname -> obfuscatedclassname:`
        let mut split = line.splitn(3, ' ');
        let original = split.next()?;
        if split.next()? != "->" || !line.ends_with(':') {
            return None;
        }
        let mut obfuscated = split.next()?;
        obfuscated = &obfuscated[..obfuscated.len() - 1];
        Some(MappingRecord::Class {
            original,
            obfuscated,
        })
    } else {
        // field line or method line:
        // `originalfieldtype originalfieldname -> obfuscatedfieldname`
        // `[startline:endline:]originalreturntype [originalclassname.]originalmethodname(originalargumenttype,...)[:originalstartline[:originalendline]] -> obfuscatedmethodname`
        line = &line[4..];
        let mut line_mapping = LineMapping::default();

        // leading line mapping
        if line.starts_with(char::is_numeric) {
            let mut nums = line.splitn(3, ':');
            line_mapping.startline = nums.next()?.parse().ok()?;
            line_mapping.endline = nums.next()?.parse().ok()?;
            line = nums.next()?;
        }
        // type
        let mut split = line.splitn(4, ' ');
        let ty = split.next()?;
        let mut original = split.next()?;
        if split.next()? != "->" {
            return None;
        }
        let obfuscated = split.next()?;

        let mut nums = original.splitn(3, ':');
        original = nums.next()?;
        line_mapping.original_startline = match nums.next() {
            Some(n) => Some(n.parse().ok()?),
            _ => None,
        };
        line_mapping.original_endline = match nums.next() {
            Some(n) => Some(n.parse().ok()?),
            _ => None,
        };
        let mut args = original.splitn(2, '(');
        original = args.next()?;
        let arguments = args.next().map(|args| &args[..args.len() - 1]);

        let mut split_class = original.rsplitn(2, '.');
        original = split_class.next()?;
        let original_class = split_class.next();

        Some(MappingRecord::Member {
            ty,
            original,
            obfuscated,
            original_class,
            arguments,
            line_mapping: if line_mapping.startline > 0 {
                Some(line_mapping)
            } else {
                None
            },
        })
    }
}
