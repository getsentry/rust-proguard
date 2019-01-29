extern crate proguard;
#[macro_use]
extern crate lazy_static;

use proguard::{MappingView, Parser};

static MAPPING_R8: &'static [u8] = include_bytes!("res/mapping-r8.txt");
lazy_static! {
    static ref MAPPING_WIN_R8: Vec<u8> = MAPPING_R8
        .iter()
        .flat_map(|&byte| if byte == b'\n' {
            vec![b'\r', b'\n']
        } else {
            vec![byte]
        })
        .collect();
}

#[test]
fn test_parse_header() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();

    let parse_result = mapping.header();
    let header = parse_result.expect("header");
    assert_eq!(header.compiler().expect("compiler"), "R8");
    assert_eq!(
        header.compiler_version().expect("compiler_version"),
        "1.3.49"
    );
    assert_eq!(header.min_api().expect("min_api"), "15");
}

#[test]
fn test_basic_r8() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    let cls = mapping.find_class("a.a.a.a.c").unwrap();

    assert_eq!(
        cls.class_name(),
        "android.arch.core.executor.ArchTaskExecutor"
    );
    assert_eq!(cls.alias(), "a.a.a.a.c");

    assert_eq!(
        &cls.get_field("b").unwrap().to_string(),
        "java.util.concurrent.Executor sMainThreadExecutor"
    );
}

#[test]
fn test_field_r8() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    let cls = mapping.find_class("a.a.a.b.c").unwrap();

    let field = cls.get_field("d").unwrap();
    assert_eq!(field.to_string(), "int mSize".to_string());
}

#[test]
fn test_methods_r8() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    let cls = mapping.find_class("a.a.a.b.c").unwrap();

    let methods = cls.get_methods("<init>", None);
    assert_eq!(methods.len(), 2);
    assert_eq!(methods[0].to_string(), "void <init>()".to_string());
}

#[test]
fn test_extra_methods() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    let cls = mapping.find_class("a.a.a.b.c$a").unwrap();
    let methods = cls.get_methods("<init>", Some(1));
    assert_eq!(methods.len(), 1);
    assert_eq!(&methods[0].to_string(),
               "void <init>(android.arch.core.internal.SafeIterableMap$Entry, android.arch.core.internal.SafeIterableMap$Entry)");
}
