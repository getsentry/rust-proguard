use std::{collections::HashMap, sync::OnceLock};

use watto::StringTable;

use crate::{ProguardMapper, ProguardMapping};

use super::raw::ProguardCache;

#[derive(Debug, Clone)]
struct ClassData<'data> {
    class_body: u32, // string table reference
    mapper: OnceLock<ProguardMapper<'data>>,
}

#[derive(Clone)]
pub struct IndexedProguard<'data> {
    string_bytes: &'data [u8],
    mappers: HashMap<&'data str, ClassData<'data>>,
}

impl<'data> IndexedProguard<'data> {
    pub fn get_mapper(&self, obfuscated_class: &str) -> Option<&ProguardMapper<'data>> {
        let class_data = self.mappers.get(obfuscated_class)?;
        let mapper = class_data.mapper.get_or_init(|| {
            let body =
                StringTable::read(self.string_bytes, class_data.class_body as usize).unwrap();
            let mapping = ProguardMapping::new(body.as_bytes());
            ProguardMapper::new(mapping)
        });

        Some(mapper)
    }
}

impl<'data> From<ProguardCache<'data>> for IndexedProguard<'data> {
    fn from(value: ProguardCache<'data>) -> Self {
        let ProguardCache {
            classes,
            string_bytes,
            ..
        } = value;

        let mut mappings = HashMap::new();

        for class in classes {
            let obfuscated =
                StringTable::read(string_bytes, class.obfuscated_name_offset as usize).unwrap();

            mappings.insert(
                obfuscated,
                ClassData {
                    class_body: class.body_offset,
                    mapper: OnceLock::new(),
                },
            );
        }

        Self {
            mappers: mappings,
            string_bytes,
        }
    }
}
