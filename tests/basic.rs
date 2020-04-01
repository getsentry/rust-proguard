use lazy_static::lazy_static;

use proguard::{MappingView, Parser};

static MAPPING: &[u8] = include_bytes!("res/mapping.txt");
lazy_static! {
    static ref MAPPING_WIN: Vec<u8> = MAPPING
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
    let mapping = MappingView::from_slice(MAPPING).unwrap();

    let parse_result = mapping.header();
    assert!(parse_result.is_none());
}

#[test]
fn test_basic() {
    let mapping = MappingView::from_slice(MAPPING).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.ConstraintLayout$a")
        .unwrap();

    assert_eq!(
        cls.class_name(),
        "android.support.constraint.ConstraintLayout$LayoutParams"
    );
    assert_eq!(cls.alias(), "android.support.constraint.ConstraintLayout$a");

    assert_eq!(&cls.get_field("b").unwrap().to_string(), "int guideEnd");
}

#[test]
fn test_basic_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN[..]).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.ConstraintLayout$a")
        .unwrap();

    assert_eq!(
        cls.class_name(),
        "android.support.constraint.ConstraintLayout$LayoutParams"
    );
    assert_eq!(cls.alias(), "android.support.constraint.ConstraintLayout$a");

    assert_eq!(&cls.get_field("b").unwrap().to_string(), "int guideEnd");
}

#[test]
fn test_methods() {
    let mapping = MappingView::from_slice(MAPPING).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.ConstraintLayout$a")
        .unwrap();

    let methods = cls.get_methods("a", Some(1848));
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].to_string(), "void validate()".to_string());
}

#[test]
fn test_methods_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN[..]).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.ConstraintLayout$a")
        .unwrap();

    let methods = cls.get_methods("a", Some(1848));
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].to_string(), "void validate()".to_string());
}

#[test]
fn test_extra_methods() {
    let mapping = MappingView::from_slice(MAPPING).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.a.e")
        .unwrap();
    let methods = cls.get_methods("a", Some(261));
    assert_eq!(methods.len(), 1);
    assert_eq!(
        &methods[0].to_string(),
        "android.support.constraint.solver.ArrayRow getRow(int)"
    );
}

#[test]
fn test_extra_methods_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN[..]).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.a.e")
        .unwrap();
    let methods = cls.get_methods("a", Some(261));
    assert_eq!(methods.len(), 1);
    assert_eq!(
        &methods[0].to_string(),
        "android.support.constraint.solver.ArrayRow getRow(int)"
    );
}

#[test]
fn test_mapping_info() {
    let mapping = MappingView::from_slice(MAPPING).unwrap();
    assert_eq!(mapping.has_line_info(), true);
}

#[test]
fn test_mapping_info_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN[..]).unwrap();
    assert_eq!(mapping.has_line_info(), true);
}

#[test]
fn test_method_matches() {
    let mapping = MappingView::from_slice(MAPPING).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.a.a")
        .unwrap();
    let meths = cls.get_methods("a", Some(320));
    assert_eq!(meths.len(), 1);
    assert_eq!(meths[0].name(), "remove");

    let meths = cls.get_methods("a", Some(200));
    assert_eq!(meths.len(), 1);
    assert_eq!(meths[0].name(), "put");
}

#[test]
fn test_method_matches_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN[..]).unwrap();
    let cls = mapping
        .find_class("android.support.constraint.a.a")
        .unwrap();
    let meths = cls.get_methods("a", Some(320));
    assert_eq!(meths.len(), 1);
    assert_eq!(meths[0].name(), "remove");

    let meths = cls.get_methods("a", Some(200));
    assert_eq!(meths.len(), 1);
    assert_eq!(meths[0].name(), "put");
}

#[test]
fn test_uuid() {
    let mapping = MappingView::from_slice(MAPPING).unwrap();
    assert_eq!(
        mapping.uuid(),
        "5cd8e873-1127-5276-81b7-8ff25043ecfd".parse().unwrap()
    );
}

#[test]
fn test_uuid_win() {
    let mapping = MappingView::from_slice(&MAPPING_WIN[..]).unwrap();
    assert_eq!(
        mapping.uuid(),
        "71d468f2-0dc4-5017-9f12-1a81081913ef".parse().unwrap()
    );
}

#[test]
fn test_iter_access() {
    let parser = Parser::from_slice(&MAPPING_WIN[..]).unwrap();
    let mut class_iter = parser.classes();
    class_iter.next();
    let cls = class_iter.next().unwrap();
    assert_eq!(cls.alias(), "android.support.constraint.ConstraintLayout");

    let mut mem_iter = cls.members();

    let mem = mem_iter.next().unwrap();
    assert_eq!(mem.alias(), "a");
    assert_eq!(mem.type_name(), "android.util.SparseArray");
    assert_eq!(mem.name(), "mChildrenByIds");
    assert!(mem.args().is_none());

    let mem = mem_iter.next().unwrap();
    assert_eq!(mem.alias(), "c");
    assert_eq!(mem.type_name(), "java.util.ArrayList");
    assert_eq!(mem.name(), "mVariableDimensionsWidgets");
    assert!(mem.args().is_none());

    let mem = (&mut mem_iter).find(|x| x.args().is_some()).unwrap();
    assert_eq!(mem.alias(), "<init>");
    assert_eq!(mem.type_name(), "void");
    assert_eq!(mem.name(), "<init>");
    assert_eq!(mem.first_line(), 395);
    assert_eq!(mem.last_line(), 416);
    assert_eq!(
        mem.args().unwrap().collect::<Vec<_>>(),
        vec!["android.content.Context"]
    );

    let mem = mem_iter.next().unwrap();
    assert_eq!(mem.alias(), "<init>");
    assert_eq!(mem.type_name(), "void");
    assert_eq!(mem.name(), "<init>");
    assert_eq!(mem.first_line(), 395);
    assert_eq!(mem.last_line(), 421);

    assert!(mem.first_line_optimized().is_none());
    assert!(mem.last_line_optimized().is_none());
    assert_eq!(
        mem.args().unwrap().collect::<Vec<_>>(),
        vec!["android.content.Context", "android.util.AttributeSet"]
    );
}

#[test]
fn test_inlined_methods() {
    // https://github.com/getsentry/rust-proguard/issues/5#issue-410310382
    let ex = br#"random -> alias:
    7:8:void method3(long):78:79 -> main
    7:8:void method2(int):87 -> main
    7:8:void method1(java.lang.String):95 -> main
    7:8:void main(java.lang.String[]):101 -> main"#;

    let mapping = MappingView::from_slice(ex).unwrap();
    let cls = mapping.find_class("alias").unwrap();
    assert_eq!(cls.members().filter(|m| m.alias() == "main").count(), 4);

    // https://github.com/getsentry/rust-proguard/issues/6#issuecomment-605610326
    let r8 = br#"com.exmaple.app.MainActivity -> com.exmaple.app.MainActivity:
    com.example1.domain.MyBean myBean -> p
    1:1:void <init>():11:11 -> <init>
    1:1:void buttonClicked(android.view.View):29:29 -> buttonClicked
    2:2:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked
    2:2:void buttonClicked(android.view.View):29 -> buttonClicked
    1:1:void onCreate(android.os.Bundle):17:17 -> onCreate
    2:5:void onCreate(android.os.Bundle):22:25 -> onCreate"#;

    let mapping = MappingView::from_slice(r8).unwrap();
    let cls = mapping.find_class("com.exmaple.app.MainActivity").unwrap();
    assert_eq!(
        cls.members()
            .filter(|m| m.alias() == "buttonClicked")
            .count(),
        3
    );

    // https://github.com/getsentry/rust-proguard/issues/6#issuecomment-605613412
    let pg = br#"com.exmaple.app.MainActivity -> com.exmaple.app.MainActivity:
    com.example1.domain.MyBean myBean -> k
    11:11:void <init>() -> <init>
    17:26:void onCreate(android.os.Bundle) -> onCreate
    29:30:void buttonClicked(android.view.View) -> buttonClicked
    1016:1016:void com.example1.domain.MyBean.doWork():16:16 -> buttonClicked
    1016:1016:void buttonClicked(android.view.View):29 -> buttonClicked"#;

    let mapping = MappingView::from_slice(pg).unwrap();
    let cls = mapping.find_class("com.exmaple.app.MainActivity").unwrap();
    assert_eq!(
        cls.members()
            .filter(|m| m.alias() == "buttonClicked")
            .count(),
        3
    );
}
