use lazy_static::lazy_static;

use proguard::{MappingView, Parser};

static MAPPING_R8: &[u8] = include_bytes!("res/mapping-r8.txt");

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

#[test]
fn test_mapping_info() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    assert_eq!(mapping.has_line_info(), true);
}

#[test]
fn test_mapping_info_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN_R8[..]).unwrap();
    assert_eq!(mapping.has_line_info(), true);
}

#[test]
fn test_method_matches() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    let cls = mapping.find_class("a.a.a.b.c").unwrap();
    let meths = cls.get_methods("a", Some(1));
    assert_eq!(meths.len(), 1);
    assert_eq!(meths[0].name(), "access$100");

    let meths = cls.get_methods("a", Some(13));
    assert_eq!(meths.len(), 1);
    assert_eq!(meths[0].name(), "eldest");
}

#[test]
fn test_uuid() {
    let mapping = MappingView::from_slice(MAPPING_R8).unwrap();
    assert_eq!(
        mapping.uuid(),
        "c96fb926-797c-53de-90ee-df2aeaf28340".parse().unwrap()
    );
}

#[test]
fn test_uuid_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN_R8[..]).unwrap();
    assert_eq!(
        mapping.uuid(),
        "d8b03b44-58df-5cd7-adc7-aefcfb0e2ade".parse().unwrap()
    );
}

#[test]
fn test_iter_access() {
    let parser = Parser::from_slice(&MAPPING_R8[..]).unwrap();
    let mut class_iter = parser.classes();
    let cls = class_iter.next().unwrap();
    assert_eq!(cls.alias(), "a.a.a.a.c");

    let mut mem_iter = cls.members();

    let mem = mem_iter.next().unwrap();
    assert_eq!(mem.alias(), "a");
    assert_eq!(
        mem.type_name(),
        "android.arch.core.executor.ArchTaskExecutor"
    );
    assert_eq!(mem.name(), "sInstance");
    assert!(mem.args().is_none());

    let mem = mem_iter.next().unwrap();
    assert_eq!(mem.alias(), "b");
    assert_eq!(mem.type_name(), "java.util.concurrent.Executor");
    assert_eq!(mem.name(), "sMainThreadExecutor");
    assert!(mem.args().is_none());

    let mem = (&mut mem_iter)
        .find(|x| x.args().is_some() && x.args().unwrap().next().is_some())
        .unwrap();
    assert_eq!(mem.alias(), "a");
    assert_eq!(mem.type_name(), "void");
    assert_eq!(mem.name(), "executeOnDiskIO");
    assert_eq!(mem.first_line(), 96);
    assert_eq!(mem.last_line(), 96);
    assert_eq!(
        mem.args().unwrap().collect::<Vec<_>>(),
        vec!["java.lang.Runnable"]
    );

    let mem = mem_iter.next().unwrap();
    assert_eq!(mem.alias(), "a");
    assert_eq!(mem.type_name(), "boolean");
    assert_eq!(mem.name(), "isMainThread");
    assert_eq!(mem.first_line(), 116);
    assert_eq!(mem.last_line(), 116);
    assert_eq!(mem.first_line_optimized().expect("lno"), 2);
    assert_eq!(mem.last_line_optimized().expect("lno"), 2);
    assert!(mem.args().unwrap().next().is_none());
}
