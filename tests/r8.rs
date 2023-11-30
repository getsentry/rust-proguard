use lazy_static::lazy_static;

use proguard::{ProguardMapper, ProguardMapping, StackFrame};

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
fn test_basic_r8() {
    let mapping = ProguardMapping::new(MAPPING_R8);
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());

    let mapper = ProguardMapper::new(mapping);

    let class = mapper.remap_class("a.a.a.a.c");
    assert_eq!(class, Some("android.arch.core.executor.ArchTaskExecutor"));
}

#[test]
fn test_extra_methods() {
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING_R8));

    let mut mapped = mapper.remap_frame(&StackFrame::new("a.a.a.b.c$a", "<init>", 1));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "android.arch.core.internal.SafeIterableMap$AscendingIterator",
            "<init>",
            270
        )
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches() {
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING_R8));

    let mut mapped = mapper.remap_frame(&StackFrame::new("a.a.a.b.c", "a", 1));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "android.arch.core.internal.SafeIterableMap",
            "access$100",
            35
        )
    );
    assert_eq!(mapped.next(), None);

    let mut mapped = mapper.remap_frame(&StackFrame::new("a.a.a.b.c", "a", 13));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new("android.arch.core.internal.SafeIterableMap", "eldest", 168)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_summary() {
    let mapping = ProguardMapping::new(MAPPING_R8);

    let summary = mapping.summary();
    assert_eq!(summary.compiler(), Some("R8"));
    assert_eq!(summary.compiler_version(), Some("1.3.49"));
    assert_eq!(summary.min_api(), Some(15));
    assert_eq!(summary.class_count(), 1167);
    assert_eq!(summary.method_count(), 24076);
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid() {
    assert_eq!(
        ProguardMapping::new(MAPPING_R8).uuid(),
        "c96fb926-797c-53de-90ee-df2aeaf28340".parse().unwrap()
    );
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid_win() {
    assert_eq!(
        ProguardMapping::new(&MAPPING_WIN_R8[..]).uuid(),
        "d8b03b44-58df-5cd7-adc7-aefcfb0e2ade".parse().unwrap()
    );
}
