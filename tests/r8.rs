use std::sync::LazyLock;

use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame};

#[cfg(feature = "uuid")]
use uuid::uuid;

static MAPPING_R8: &[u8] = include_bytes!("res/mapping-r8.txt");
static MAPPING_R8_SYMBOLICATED_FILE_NAMES: &[u8] =
    include_bytes!("res/mapping-r8-symbolicated_file_names.txt");
static MAPPING_OUTLINE: &[u8] = include_bytes!("res/mapping-outline.txt");
static MAPPING_OUTLINE_COMPLEX: &[u8] = include_bytes!("res/mapping-outline-complex.txt");
static MAPPING_REWRITE_COMPLEX: &str = include_str!("res/mapping-rewrite-complex.txt");

static MAPPING_WIN_R8: LazyLock<Vec<u8>> = LazyLock::new(|| {
    MAPPING_R8
        .iter()
        .flat_map(|&byte| {
            if byte == b'\n' {
                vec![b'\r', b'\n']
            } else {
                vec![byte]
            }
        })
        .collect()
});

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
        uuid!("c96fb926-797c-53de-90ee-df2aeaf28340")
    );
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid_win() {
    assert_eq!(
        ProguardMapping::new(&MAPPING_WIN_R8[..]).uuid(),
        uuid!("d8b03b44-58df-5cd7-adc7-aefcfb0e2ade")
    );
}

#[test]
fn test_remap_source_file() {
    let mapping = ProguardMapping::new(MAPPING_R8_SYMBOLICATED_FILE_NAMES);

    let mapper = ProguardMapper::new(mapping);

    let test = mapper.remap_stacktrace(
        r#"
    Caused by: java.lang.Exception: Hello from main!
	at a.a.a(SourceFile:12)
	at io.wzieba.r8fullmoderenamessources.MainActivity.b(SourceFile:6)
	at io.wzieba.r8fullmoderenamessources.MainActivity.a(SourceFile:1)
	at a.c.onClick(SourceFile:1)
	at android.view.View.performClick(View.java:7659)
	at android.view.View.performClickInternal(View.java:7636)
	at android.view.View.-$$Nest$mperformClickInternal(Unknown Source:0)"#,
    );

    assert_eq!(r#"
    Caused by: java.lang.Exception: Hello from main!
    at io.wzieba.r8fullmoderenamessources.Foobar.foo(Foobar.kt:10)
    at io.wzieba.r8fullmoderenamessources.MainActivity.onCreate$lambda$1$lambda$0(MainActivity.kt:14)
    at io.wzieba.r8fullmoderenamessources.MainActivity.$r8$lambda$pOQDVg57r6gG0-DzwbGf17BfNbs(MainActivity.kt:0)
    at io.wzieba.r8fullmoderenamessources.MainActivity$$ExternalSyntheticLambda0.onClick(MainActivity:0)
	at android.view.View.performClick(View.java:7659)
	at android.view.View.performClickInternal(View.java:7636)
	at android.view.View.-$$Nest$mperformClickInternal(Unknown Source:0)"#.trim(), test.unwrap().trim());
}

#[test]
fn test_remap_outlines() {
    let mapping = ProguardMapping::new(MAPPING_OUTLINE_COMPLEX);

    let mapper = ProguardMapper::new(mapping);

    let test = mapper.remap_stacktrace(
        r#"
    java.lang.IllegalStateException: Oops!
    at ev.h.b(SourceFile:3)
    at uu0.k.l(SourceFile:43)
    at b80.f.a(SourceFile:33)
    at er3.f.invoke(SourceFile:3)
    at yv0.g.d(SourceFile:17)
    at er3.g$a.invoke(SourceFile:36)
    at h1.p0.d(SourceFile:5)
    at p1.k.c(SourceFile:135)
    at h1.y.A(SourceFile:111)
    at h1.y.m(SourceFile:6)
    at h1.e3.invoke(SourceFile:231)
    at w2.r0$c.doFrame(SourceFile:7)
    at w2.q0$c.doFrame(SourceFile:48)
    at android.view.Choreographer$CallbackRecord.run(Choreographer.java:1899)"#,
    );

    assert_eq!(r#"
    java.lang.IllegalStateException: Oops!
    at com.example.projection.MapProjectionViewController.onProjectionView(MapProjectionViewController.kt:160)
    at com.example.projection.MapProjectionViewController.createProjectionMarkerInternal(MapProjectionViewController.kt:133)
    at com.example.projection.MapProjectionViewController.createProjectionMarker(MapProjectionViewController.kt:79)
    at com.example.MapAnnotations.createProjectionMarker(MapAnnotations.kt:63)
    at com.example.mapcomponents.marker.currentlocation.DotRendererDelegate.createCurrentLocationProjectionMarker(DotRendererDelegate.kt:101)
    at com.example.mapcomponents.marker.currentlocation.DotRendererDelegate.render(DotRendererDelegate.kt:34)
    at com.example.mapcomponents.marker.currentlocation.CurrentLocationRenderer.render(CurrentLocationRenderer.kt:39)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1$mapReadyCallback$1.invoke(CurrentLocationMarkerMapCollection.kt:36)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1$mapReadyCallback$1.invoke(CurrentLocationMarkerMapCollection.kt:36)
    at com.example.mapbox.MapboxMapView.addMapReadyCallback(MapboxMapView.kt:368)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1.invoke(CurrentLocationMarkerMapCollection.kt:40)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1.invoke(CurrentLocationMarkerMapCollection.kt:35)
    at androidx.compose.runtime.DisposableEffectImpl.onRemembered(Effects.kt:85)
    at androidx.compose.runtime.internal.RememberEventDispatcher.dispatchRememberList(RememberEventDispatcher.kt:253)
    at androidx.compose.runtime.internal.RememberEventDispatcher.dispatchRememberObservers(RememberEventDispatcher.kt:225)
    at androidx.compose.runtime.CompositionImpl.applyChangesInLocked(Composition.kt:1122)
    at androidx.compose.runtime.CompositionImpl.applyChanges(Composition.kt:1149)
    at androidx.compose.runtime.Recomposer$runRecomposeAndApplyChanges$2.invokeSuspend$lambda$22(Recomposer.kt:705)
    at androidx.compose.ui.platform.AndroidUiFrameClock$withFrameNanos$2$callback$1.doFrame(AndroidUiFrameClock.android.kt:39)
    at androidx.compose.ui.platform.AndroidUiDispatcher.performFrameDispatch(AndroidUiDispatcher.android.kt:108)
    at androidx.compose.ui.platform.AndroidUiDispatcher.access$performFrameDispatch(AndroidUiDispatcher.android.kt:41)
    at androidx.compose.ui.platform.AndroidUiDispatcher$dispatchCallback$1.doFrame(AndroidUiDispatcher.android.kt:69)
    at android.view.Choreographer$CallbackRecord.run(Choreographer.java:1899)"#.trim(), test.unwrap().trim());
}

#[test]
fn test_remap_outlines_cache() {
    let mapping = ProguardMapping::new(MAPPING_OUTLINE_COMPLEX);

    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let test = cache.remap_stacktrace(
        r#"
    java.lang.IllegalStateException: Oops!
    at ev.h.b(SourceFile:3)
    at uu0.k.l(SourceFile:43)
    at b80.f.a(SourceFile:33)
    at er3.f.invoke(SourceFile:3)
    at yv0.g.d(SourceFile:17)
    at er3.g$a.invoke(SourceFile:36)
    at h1.p0.d(SourceFile:5)
    at p1.k.c(SourceFile:135)
    at h1.y.A(SourceFile:111)
    at h1.y.m(SourceFile:6)
    at h1.e3.invoke(SourceFile:231)
    at w2.r0$c.doFrame(SourceFile:7)
    at w2.q0$c.doFrame(SourceFile:48)
    at android.view.Choreographer$CallbackRecord.run(Choreographer.java:1899)"#,
    );

    assert_eq!(r#"
    java.lang.IllegalStateException: Oops!
    at com.example.projection.MapProjectionViewController.onProjectionView(MapProjectionViewController.kt:160)
    at com.example.projection.MapProjectionViewController.createProjectionMarkerInternal(MapProjectionViewController.kt:133)
    at com.example.projection.MapProjectionViewController.createProjectionMarker(MapProjectionViewController.kt:79)
    at com.example.MapAnnotations.createProjectionMarker(MapAnnotations.kt:63)
    at com.example.mapcomponents.marker.currentlocation.DotRendererDelegate.createCurrentLocationProjectionMarker(DotRendererDelegate.kt:101)
    at com.example.mapcomponents.marker.currentlocation.DotRendererDelegate.render(DotRendererDelegate.kt:34)
    at com.example.mapcomponents.marker.currentlocation.CurrentLocationRenderer.render(CurrentLocationRenderer.kt:39)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1$mapReadyCallback$1.invoke(CurrentLocationMarkerMapCollection.kt:36)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1$mapReadyCallback$1.invoke(CurrentLocationMarkerMapCollection.kt:36)
    at com.example.mapbox.MapboxMapView.addMapReadyCallback(MapboxMapView.kt:368)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1.invoke(CurrentLocationMarkerMapCollection.kt:40)
    at com.example.map.internal.CurrentLocationMarkerMapCollectionKt$CurrentLocationMarkerMapCollection$1$1.invoke(CurrentLocationMarkerMapCollection.kt:35)
    at androidx.compose.runtime.DisposableEffectImpl.onRemembered(Effects.kt:85)
    at androidx.compose.runtime.internal.RememberEventDispatcher.dispatchRememberList(RememberEventDispatcher.kt:253)
    at androidx.compose.runtime.internal.RememberEventDispatcher.dispatchRememberObservers(RememberEventDispatcher.kt:225)
    at androidx.compose.runtime.CompositionImpl.applyChangesInLocked(Composition.kt:1122)
    at androidx.compose.runtime.CompositionImpl.applyChanges(Composition.kt:1149)
    at androidx.compose.runtime.Recomposer$runRecomposeAndApplyChanges$2.invokeSuspend$lambda$22(Recomposer.kt:705)
    at androidx.compose.ui.platform.AndroidUiFrameClock$withFrameNanos$2$callback$1.doFrame(AndroidUiFrameClock.android.kt:39)
    at androidx.compose.ui.platform.AndroidUiDispatcher.performFrameDispatch(AndroidUiDispatcher.android.kt:108)
    at androidx.compose.ui.platform.AndroidUiDispatcher.access$performFrameDispatch(AndroidUiDispatcher.android.kt:41)
    at androidx.compose.ui.platform.AndroidUiDispatcher$dispatchCallback$1.doFrame(AndroidUiDispatcher.android.kt:69)
    at android.view.Choreographer$CallbackRecord.run(Choreographer.java:1899)"#.trim(), test.unwrap().trim());
}

#[test]
fn test_outline_header_parsing_cache() {
    let mapping = ProguardMapping::new(MAPPING_OUTLINE);
    assert!(mapping.is_valid());

    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    // Test that we can remap the outline class
    let class = cache.remap_class("a");
    assert_eq!(class, Some("outline.Class"));

    // Test that we can remap the other class
    let class = cache.remap_class("b");
    assert_eq!(class, Some("some.Class"));
}

#[test]
fn test_outline_frame_retracing_cache() {
    let mapping = ProguardMapping::new(MAPPING_OUTLINE);

    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    // Test retracing a frame from the outline class
    let mut mapped = cache.remap_frame(&StackFrame::new("a", "a", 1));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new("outline.Class", "outline", 1)
    );
    assert_eq!(mapped.next(), None);

    // Test retracing a frame from the class with outlineCallsite
    let mut mapped = cache.remap_frame(&StackFrame::new("b", "s", 27));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new("some.Class", "outlineCaller", 0)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_outline_header_parsing() {
    let mapping = ProguardMapping::new(MAPPING_OUTLINE);
    assert!(mapping.is_valid());

    let mapper = ProguardMapper::new(mapping);

    // Test that we can remap the outline class
    let class = mapper.remap_class("a");
    assert_eq!(class, Some("outline.Class"));

    // Test that we can remap the other class
    let class = mapper.remap_class("b");
    assert_eq!(class, Some("some.Class"));
}

#[test]
fn test_outline_frame_retracing() {
    let mapping = ProguardMapping::new(MAPPING_OUTLINE);
    let mapper = ProguardMapper::new(mapping);

    // Test retracing a frame from the outline class
    let mut mapped = mapper.remap_frame(&StackFrame::new("a", "a", 1));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new("outline.Class", "outline", 1)
    );
    assert_eq!(mapped.next(), None);

    // Test retracing a frame from the class with outlineCallsite
    let mut mapped = mapper.remap_frame(&StackFrame::new("b", "s", 27));

    assert_eq!(
        mapped.next().unwrap(),
        StackFrame::new("some.Class", "outlineCaller", 0)
    );
    assert_eq!(mapped.next(), None);
}

#[test]
fn rewrite_frame_complex_stacktrace() {
    let mapper = ProguardMapper::from(MAPPING_REWRITE_COMPLEX);

    let input = "\
java.lang.NullPointerException: Primary issue
    at a.start(SourceFile:10)
    at b.dispatch(SourceFile:5)
    at c.draw(SourceFile:20)
Caused by: java.lang.IllegalStateException: Secondary issue
    at b.dispatch(SourceFile:5)
    at c.draw(SourceFile:20)
";

    let expected = "\
java.lang.NullPointerException: Primary issue
    at com.example.flow.Initializer.start(SourceFile:42)
    at com.example.flow.StreamRouter$Inline.internalDispatch(<unknown>:30)
    at com.example.flow.StreamRouter.dispatch(SourceFile:12)
    at com.example.flow.UiBridge.render(SourceFile:200)
Caused by: java.lang.IllegalStateException: Secondary issue
    at com.example.flow.StreamRouter.dispatch(SourceFile:12)
    at com.example.flow.UiBridge.render(SourceFile:200)
";

    let actual = mapper.remap_stacktrace(input).unwrap();
    assert_eq!(actual, expected);
}
