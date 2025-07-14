use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame};

static MAPPING_CALLBACK: &[u8] = include_bytes!("res/mapping-callback.txt");
static MAPPING_CALLBACK_EXTRA_CLASS: &[u8] = include_bytes!("res/mapping-callback-extra-class.txt");
static MAPPING_CALLBACK_INNER_CLASS: &[u8] = include_bytes!("res/mapping-callback-inner-class.txt");

#[test]
fn test_method_matches_callback_mapper() {
    // see the following files for sources used when creating the mapping file:
    //   - res/mapping-callback_EditActivity.kt
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING_CALLBACK));

    let mut mapped = mapper.remap_frame(&StackFrame::new(
        "io.sentry.samples.instrumentation.ui.g",
        "onMenuItemClick",
        28,
    ));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity",
            "onCreate$lambda$1",
            37,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity$$InternalSyntheticLambda$1$ebaa538726b99bb77e0f5e7c86443911af17d6e5be2b8771952ae0caa4ff2ac7$0",
            "onMenuItemClick",
            0,
        )
        .with_method_synthesized(true)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches_callback_cache() {
    // see the following files for sources used when creating the mapping file:
    //   - res/mapping-callback_EditActivity.kt
    let mapping = ProguardMapping::new(MAPPING_CALLBACK);
    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();

    let cache = ProguardCache::parse(&cache).unwrap();

    cache.test();

    let mut mapped = cache.remap_frame(&StackFrame::new(
        "io.sentry.samples.instrumentation.ui.g",
        "onMenuItemClick",
        28,
    ));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity",
            "onCreate$lambda$1",
            37,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity$$InternalSyntheticLambda$1$ebaa538726b99bb77e0f5e7c86443911af17d6e5be2b8771952ae0caa4ff2ac7$0",
            "onMenuItemClick",
            0,
        )
        .with_method_synthesized(true)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches_callback_extra_class_mapper() {
    // see the following files for sources used when creating the mapping file:
    //   - res/mapping-callback-extra-class_EditActivity.kt
    //   - res/mapping-callback-extra-class_TestSourceContext.kt
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING_CALLBACK_EXTRA_CLASS));

    let mut mapped = mapper.remap_frame(&StackFrame::new(
        "io.sentry.samples.instrumentation.ui.g",
        "onMenuItemClick",
        28,
    ));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.TestSourceContext",
            "test2",
            10,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.TestSourceContext",
            "test",
            6,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity",
            "onCreate$lambda$1",
            38,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity$$InternalSyntheticLambda$1$ebaa538726b99bb77e0f5e7c86443911af17d6e5be2b8771952ae0caa4ff2ac7$0",
            "onMenuItemClick",
            0,
        )
        .with_method_synthesized(true)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches_callback_extra_class_cache() {
    // see the following files for sources used when creating the mapping file:
    //   - res/mapping-callback-extra-class_EditActivity.kt
    //   - res/mapping-callback-extra-class_TestSourceContext.kt
    let mapping = ProguardMapping::new(MAPPING_CALLBACK_EXTRA_CLASS);
    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();

    let cache = ProguardCache::parse(&cache).unwrap();

    cache.test();

    let mut mapped = cache.remap_frame(&StackFrame::new(
        "io.sentry.samples.instrumentation.ui.g",
        "onMenuItemClick",
        28,
    ));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.TestSourceContext",
            "test2",
            10,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.TestSourceContext",
            "test",
            6,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity",
            "onCreate$lambda$1",
            38,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity$$InternalSyntheticLambda$1$ebaa538726b99bb77e0f5e7c86443911af17d6e5be2b8771952ae0caa4ff2ac7$0",
            "onMenuItemClick",
            0,
        )
        .with_method_synthesized(true)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches_callback_inner_class_mapper() {
    // see the following files for sources used when creating the mapping file:
    //   - res/mapping-callback-inner-class_EditActivity.kt
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING_CALLBACK_INNER_CLASS));

    let mut mapped = mapper.remap_frame(&StackFrame::new(
        "io.sentry.samples.instrumentation.ui.g",
        "onMenuItemClick",
        28,
    ));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::with_file(
            "io.sentry.samples.instrumentation.ui.EditActivity$InnerEditActivityClass",
            "testInner",
            19,
            "EditActivity.kt",
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity",
            "onCreate$lambda$1",
            45,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity$$InternalSyntheticLambda$1$ebaa538726b99bb77e0f5e7c86443911af17d6e5be2b8771952ae0caa4ff2ac7$0",
            "onMenuItemClick",
            0,
        )
        .with_method_synthesized(true)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_matches_callback_inner_class_cache() {
    // see the following files for sources used when creating the mapping file:
    //   - res/mapping-callback-inner-class_EditActivity.kt
    let mapping = ProguardMapping::new(MAPPING_CALLBACK_INNER_CLASS);
    let mut cache = Vec::new();
    ProguardCache::write(&mapping, &mut cache).unwrap();

    let cache = ProguardCache::parse(&cache).unwrap();

    cache.test();

    let mut mapped = cache.remap_frame(&StackFrame::new(
        "io.sentry.samples.instrumentation.ui.g",
        "onMenuItemClick",
        28,
    ));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::with_file(
            "io.sentry.samples.instrumentation.ui.EditActivity$InnerEditActivityClass",
            "testInner",
            19,
            "EditActivity.kt",
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity",
            "onCreate$lambda$1",
            45,
        )
    );
    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new(
            "io.sentry.samples.instrumentation.ui.EditActivity$$InternalSyntheticLambda$1$ebaa538726b99bb77e0f5e7c86443911af17d6e5be2b8771952ae0caa4ff2ac7$0",
            "onMenuItemClick",
            0,
        )
        .with_method_synthesized(true)
    );
    assert_eq!(mapped.next(), None);
}
