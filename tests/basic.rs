use lazy_static::lazy_static;

use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame};

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
    let mapping = ProguardMapping::new(MAPPING);
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());

    let mapper = ProguardMapper::new(mapping);

    let class = mapper.remap_class("android.support.constraint.ConstraintLayout$a");
    assert_eq!(
        class,
        Some("android.support.constraint.ConstraintLayout$LayoutParams")
    );
}

#[test]
fn test_basic_cache() {
    let mapping = ProguardMapping::new(MAPPING);
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());

    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();
    let cache = ProguardCache::parse(&cache).unwrap();

    let class = cache.remap_class("android.support.constraint.ConstraintLayout$a");
    assert_eq!(
        class,
        Some("android.support.constraint.ConstraintLayout$LayoutParams")
    );
}

#[test]
fn test_basic_win() {
    let mapping = ProguardMapping::new(&MAPPING_WIN[..]);
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());

    let mapper = ProguardMapper::new(mapping);

    let class = mapper.remap_class("android.support.constraint.ConstraintLayout$a");
    assert_eq!(
        class,
        Some("android.support.constraint.ConstraintLayout$LayoutParams")
    );
}

#[test]
fn test_basic_win_cache() {
    let mapping = ProguardMapping::new(&MAPPING_WIN[..]);
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());

    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();
    let cache = ProguardCache::parse(&cache).unwrap();

    let class = cache.remap_class("android.support.constraint.ConstraintLayout$a");
    assert_eq!(
        class,
        Some("android.support.constraint.ConstraintLayout$LayoutParams")
    );
}

#[test]
fn test_method_matches() {
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING));

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
fn test_method_matches_cache() {
    let mapping = ProguardMapping::new(MAPPING);

    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();
    let cache = ProguardCache::parse(&cache).unwrap();

    let mut mapped =
        cache.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 320));

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
        cache.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 200));

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
    let mapper = ProguardMapper::new(ProguardMapping::new(&MAPPING_WIN[..]));

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
fn test_method_matches_win_cache() {
    let mapping = ProguardMapping::new(&MAPPING_WIN[..]);

    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();
    let cache = ProguardCache::parse(&cache).unwrap();

    let mut mapped =
        cache.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 320));

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
        cache.remap_frame(&StackFrame::new("android.support.constraint.a.a", "a", 200));

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
fn test_inlines() {
    let mapping = ProguardMapping::new(include_bytes!("res/mapping-inlines.txt"));
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());
    #[cfg(feature = "uuid")]
    {
        assert_eq!(
            mapping.uuid(),
            "3828bd45-950f-5e77-9737-b6b3a1d80299".parse().unwrap()
        );
    }

    let mapper = ProguardMapper::new(mapping);

    let raw = r#"java.lang.RuntimeException: Button press caused an exception!
    at io.sentry.sample.MainActivity.t(MainActivity.java:1)
    at e.a.c.a.onClick
    at android.view.View.performClick(View.java:7125)
    at android.view.View.performClickInternal(View.java:7102)
    at android.view.View.access$3500(View.java:801)
    at android.view.View$PerformClick.run(View.java:27336)
    at android.os.Handler.handleCallback(Handler.java:883)
    at android.os.Handler.dispatchMessage(Handler.java:100)
    at android.os.Looper.loop(Looper.java:214)
    at android.app.ActivityThread.main(ActivityThread.java:7356)
    at java.lang.reflect.Method.invoke(Method.java)
    at com.android.internal.os.RuntimeInit$MethodAndArgsCaller.run(RuntimeInit.java:492)
    at com.android.internal.os.ZygoteInit.main(ZygoteInit.java:930)"#;
    let remapped = mapper.remap_stacktrace(raw).unwrap();

    assert_eq!(
        remapped.trim(),
        r#"java.lang.RuntimeException: Button press caused an exception!
    at io.sentry.sample.MainActivity.bar(MainActivity.java:54)
    at io.sentry.sample.MainActivity.foo(MainActivity.java:44)
    at io.sentry.sample.MainActivity.onClickHandler(MainActivity.java:40)
    at e.a.c.a.onClick
    at android.view.View.performClick(View.java:7125)
    at android.view.View.performClickInternal(View.java:7102)
    at android.view.View.access$3500(View.java:801)
    at android.view.View$PerformClick.run(View.java:27336)
    at android.os.Handler.handleCallback(Handler.java:883)
    at android.os.Handler.dispatchMessage(Handler.java:100)
    at android.os.Looper.loop(Looper.java:214)
    at android.app.ActivityThread.main(ActivityThread.java:7356)
    at java.lang.reflect.Method.invoke(Method.java)
    at com.android.internal.os.RuntimeInit$MethodAndArgsCaller.run(RuntimeInit.java:492)
    at com.android.internal.os.ZygoteInit.main(ZygoteInit.java:930)"#
    );
}

#[test]
fn test_inlines_cache() {
    let mapping = ProguardMapping::new(include_bytes!("res/mapping-inlines.txt"));
    assert!(mapping.is_valid());
    assert!(mapping.has_line_info());
    #[cfg(feature = "uuid")]
    {
        assert_eq!(
            mapping.uuid(),
            "3828bd45-950f-5e77-9737-b6b3a1d80299".parse().unwrap()
        );
    }

    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();
    let cache = ProguardCache::parse(&cache).unwrap();

    let raw = r#"java.lang.RuntimeException: Button press caused an exception!
    at io.sentry.sample.MainActivity.t(MainActivity.java:1)
    at e.a.c.a.onClick
    at android.view.View.performClick(View.java:7125)
    at android.view.View.performClickInternal(View.java:7102)
    at android.view.View.access$3500(View.java:801)
    at android.view.View$PerformClick.run(View.java:27336)
    at android.os.Handler.handleCallback(Handler.java:883)
    at android.os.Handler.dispatchMessage(Handler.java:100)
    at android.os.Looper.loop(Looper.java:214)
    at android.app.ActivityThread.main(ActivityThread.java:7356)
    at java.lang.reflect.Method.invoke(Method.java)
    at com.android.internal.os.RuntimeInit$MethodAndArgsCaller.run(RuntimeInit.java:492)
    at com.android.internal.os.ZygoteInit.main(ZygoteInit.java:930)"#;
    let remapped = cache.remap_stacktrace(raw).unwrap();

    assert_eq!(
        remapped.trim(),
        r#"java.lang.RuntimeException: Button press caused an exception!
    at io.sentry.sample.MainActivity.bar(MainActivity.java:54)
    at io.sentry.sample.MainActivity.foo(MainActivity.java:44)
    at io.sentry.sample.MainActivity.onClickHandler(MainActivity.java:40)
    at e.a.c.a.onClick
    at android.view.View.performClick(View.java:7125)
    at android.view.View.performClickInternal(View.java:7102)
    at android.view.View.access$3500(View.java:801)
    at android.view.View$PerformClick.run(View.java:27336)
    at android.os.Handler.handleCallback(Handler.java:883)
    at android.os.Handler.dispatchMessage(Handler.java:100)
    at android.os.Looper.loop(Looper.java:214)
    at android.app.ActivityThread.main(ActivityThread.java:7356)
    at java.lang.reflect.Method.invoke(Method.java)
    at com.android.internal.os.RuntimeInit$MethodAndArgsCaller.run(RuntimeInit.java:492)
    at com.android.internal.os.ZygoteInit.main(ZygoteInit.java:930)"#
    );
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid() {
    assert_eq!(
        ProguardMapping::new(MAPPING).uuid(),
        "5cd8e873-1127-5276-81b7-8ff25043ecfd".parse().unwrap()
    );
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid_win() {
    assert_eq!(
        ProguardMapping::new(&MAPPING_WIN[..]).uuid(),
        "71d468f2-0dc4-5017-9f12-1a81081913ef".parse().unwrap()
    );
}
