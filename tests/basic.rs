use lazy_static::lazy_static;

use proguard::{Mapper, StackFrame};

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
fn test_basic() {
    let mapper = Mapper::new(MAPPING);

    let class = mapper.remap_class("android.support.constraint.ConstraintLayout$a");
    assert_eq!(
        class,
        Some("android.support.constraint.ConstraintLayout$LayoutParams")
    );
}

#[test]
fn test_basic_win() {
    let mapper = Mapper::new(&MAPPING_WIN[..]);

    let class = mapper.remap_class("android.support.constraint.ConstraintLayout$a");
    assert_eq!(
        class,
        Some("android.support.constraint.ConstraintLayout$LayoutParams")
    );
}

#[test]
fn test_method_matches() {
    let mapper = Mapper::new(MAPPING);

    let mut mapped =
        mapper.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 320));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "android.support.constraint.solver.ArrayLinkedVariables",
            "remove",
            320
        )
    );
    assert_eq!(mapped.next(), None);

    let mut mapped =
        mapper.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 200));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "android.support.constraint.solver.ArrayLinkedVariables",
            "put",
            200
        )
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches_win() {
    let mapper = Mapper::new(&MAPPING_WIN[..]);

    let mut mapped =
        mapper.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 320));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "android.support.constraint.solver.ArrayLinkedVariables",
            "remove",
            320
        )
    );
    assert_eq!(mapped.next(), None);

    let mut mapped =
        mapper.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 200));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "android.support.constraint.solver.ArrayLinkedVariables",
            "put",
            200
        )
    );
    assert_eq!(mapped.next(), None);
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid() {
    assert_eq!(
        proguard::mapping_uuid(MAPPING),
        "5cd8e873-1127-5276-81b7-8ff25043ecfd".parse().unwrap()
    );
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid_win() {
    assert_eq!(
        proguard::mapping_uuid(&MAPPING_WIN[..]),
        "71d468f2-0dc4-5017-9f12-1a81081913ef".parse().unwrap()
    );
}
