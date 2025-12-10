use std::sync::LazyLock;

use proguard::{ProguardCache, ProguardMapper, ProguardMapping, StackFrame, StackTrace, Throwable};

#[cfg(feature = "uuid")]
use uuid::uuid;

static MAPPING_R8: &[u8] = include_bytes!("res/mapping-r8.txt");
static MAPPING_R8_SYMBOLICATED_FILE_NAMES: &[u8] =
    include_bytes!("res/mapping-r8-symbolicated_file_names.txt");
static MAPPING_OUTLINE: &[u8] = include_bytes!("res/mapping-outline.txt");
static MAPPING_OUTLINE_COMPLEX: &[u8] = include_bytes!("res/mapping-outline-complex.txt");
static MAPPING_REWRITE_COMPLEX: &str = include_str!("res/mapping-rewrite-complex.txt");
static MAPPING_ZERO_LINE_INFO: &[u8] = include_bytes!("res/mapping-zero-line-info.txt");

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

#[test]
fn rewrite_frame_complex_stacktrace_cache() {
    let mut cache_bytes = Vec::new();
    ProguardCache::write(
        &ProguardMapping::new(MAPPING_REWRITE_COMPLEX.as_bytes()),
        &mut cache_bytes,
    )
    .unwrap();
    let cache = ProguardCache::parse(&cache_bytes).unwrap();
    cache.test();

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

    let actual = cache.remap_stacktrace(input).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn rewrite_frame_complex_stacktrace_typed() {
    let mapper = ProguardMapper::from(MAPPING_REWRITE_COMPLEX);

    let trace = StackTrace::with_cause(
        Some(Throwable::with_message(
            "java.lang.NullPointerException",
            "Primary issue",
        )),
        vec![
            StackFrame::with_file("a", "start", 10, "SourceFile"),
            StackFrame::with_file("b", "dispatch", 5, "SourceFile"),
            StackFrame::with_file("c", "draw", 20, "SourceFile"),
        ],
        StackTrace::new(
            Some(Throwable::with_message(
                "java.lang.IllegalStateException",
                "Secondary issue",
            )),
            vec![
                StackFrame::with_file("b", "dispatch", 5, "SourceFile"),
                StackFrame::with_file("c", "draw", 20, "SourceFile"),
            ],
        ),
    );

    let remapped = mapper.remap_stacktrace_typed(&trace);

    // After rewrite rule removes 2 inner frames for NullPointerException
    let frames = remapped.frames();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].class(), "com.example.flow.Initializer");
    assert_eq!(frames[0].method(), "start");
    assert_eq!(frames[0].line(), 42);
    assert_eq!(frames[1].class(), "com.example.flow.StreamRouter$Inline");
    assert_eq!(frames[1].method(), "internalDispatch");
    assert_eq!(frames[1].line(), 30);
    assert_eq!(frames[2].class(), "com.example.flow.StreamRouter");
    assert_eq!(frames[2].method(), "dispatch");
    assert_eq!(frames[2].line(), 12);
    assert_eq!(frames[3].class(), "com.example.flow.UiBridge");
    assert_eq!(frames[3].method(), "render");
    assert_eq!(frames[3].line(), 200);

    // Caused by exception (also not in mapping)
    let cause = remapped.cause().unwrap();
    assert!(cause.exception().is_none());

    // After rewrite rule removes 1 inner frame for IllegalStateException
    let cause_frames = cause.frames();
    assert_eq!(cause_frames.len(), 2);
    assert_eq!(cause_frames[0].class(), "com.example.flow.StreamRouter");
    assert_eq!(cause_frames[0].method(), "dispatch");
    assert_eq!(cause_frames[0].line(), 12);
    assert_eq!(cause_frames[1].class(), "com.example.flow.UiBridge");
    assert_eq!(cause_frames[1].method(), "render");
    assert_eq!(cause_frames[1].line(), 200);
}

#[test]
fn test_remap_zero_line_info() {
    let mapping = ProguardMapping::new(MAPPING_ZERO_LINE_INFO);

    let mapper = ProguardMapper::new(mapping);

    let test = mapper.remap_stacktrace(
        r#"
    java.lang.IllegalStateException: Oops!
    at id2.b.g(:18)
    at id2.b.e(:10)
    at id2.b.d(:23)
    at jb2.e.d(:7)
    at u20.c.c(:17)
    at u20.c.a(:3)
    at u20.b.a(:16)
    at u20.a$a.accept(:17)
    at ee3.l.onNext(:8)
    at je3.e1$a.i(:47)
    at je3.e1$a.run(:8)
    at wd3.b$b.run(:2)"#,
    );

    assert_eq!(r#"
    java.lang.IllegalStateException: Oops!
    at com.example.maps.projection.MapProjectionViewController.onProjectionView(MapProjectionViewController.kt:160)
    at com.example.maps.projection.MapProjectionViewController.createProjectionMarkerInternal(MapProjectionViewController.kt:133)
    at com.example.maps.projection.MapProjectionViewController.createProjectionMarker(MapProjectionViewController.kt:79)
    at com.example.maps.MapAnnotations.createProjectionMarker(MapAnnotations.kt:63)
    at com.example.design.mapcomponents.marker.currentlocation.DotRendererDelegate.createCurrentLocationProjectionMarker(DotRendererDelegate.kt:101)
    at com.example.design.mapcomponents.marker.currentlocation.DotRendererDelegate.render(DotRendererDelegate.kt:34)
    at com.example.design.mapcomponents.marker.currentlocation.CurrentLocationRenderer.render(CurrentLocationRenderer.kt:39)
    at com.example.design.mapcomponents.marker.currentlocation.CurrentLocationMarkerMapController$onMapAttach$$inlined$bindStream$1.accept(RxExt.kt:221)
    at io.reactivex.internal.observers.LambdaObserver.onNext(LambdaObserver.java:63)
    at io.reactivex.internal.operators.observable.ObservableObserveOn$ObserveOnObserver.drainNormal(ObservableObserveOn.java:201)
    at io.reactivex.internal.operators.observable.ObservableObserveOn$ObserveOnObserver.run(ObservableObserveOn.java:255)
    at io.reactivex.android.schedulers.HandlerScheduler$ScheduledRunnable.run(HandlerScheduler.java:124)"#.trim(), test.unwrap().trim());
}

#[test]
fn test_remap_zero_line_info_cache() {
    let mapping = ProguardMapping::new(MAPPING_ZERO_LINE_INFO);

    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    let test = cache.remap_stacktrace(
        r#"
    java.lang.IllegalStateException: Oops!
    at id2.b.g(:18)
    at id2.b.e(:10)
    at id2.b.d(:23)
    at jb2.e.d(:7)
    at u20.c.c(:17)
    at u20.c.a(:3)
    at u20.b.a(:16)
    at u20.a$a.accept(:17)
    at ee3.l.onNext(:8)
    at je3.e1$a.i(:47)
    at je3.e1$a.run(:8)
    at wd3.b$b.run(:2)"#,
    );

    assert_eq!(r#"
    java.lang.IllegalStateException: Oops!
    at com.example.maps.projection.MapProjectionViewController.onProjectionView(MapProjectionViewController.kt:160)
    at com.example.maps.projection.MapProjectionViewController.createProjectionMarkerInternal(MapProjectionViewController.kt:133)
    at com.example.maps.projection.MapProjectionViewController.createProjectionMarker(MapProjectionViewController.kt:79)
    at com.example.maps.MapAnnotations.createProjectionMarker(MapAnnotations.kt:63)
    at com.example.design.mapcomponents.marker.currentlocation.DotRendererDelegate.createCurrentLocationProjectionMarker(DotRendererDelegate.kt:101)
    at com.example.design.mapcomponents.marker.currentlocation.DotRendererDelegate.render(DotRendererDelegate.kt:34)
    at com.example.design.mapcomponents.marker.currentlocation.CurrentLocationRenderer.render(CurrentLocationRenderer.kt:39)
    at com.example.design.mapcomponents.marker.currentlocation.CurrentLocationMarkerMapController$onMapAttach$$inlined$bindStream$1.accept(RxExt.kt:221)
    at io.reactivex.internal.observers.LambdaObserver.onNext(LambdaObserver.java:63)
    at io.reactivex.internal.operators.observable.ObservableObserveOn$ObserveOnObserver.drainNormal(ObservableObserveOn.java:201)
    at io.reactivex.internal.operators.observable.ObservableObserveOn$ObserveOnObserver.run(ObservableObserveOn.java:255)
    at io.reactivex.android.schedulers.HandlerScheduler$ScheduledRunnable.run(HandlerScheduler.java:124)"#.trim(), test.unwrap().trim());
}

#[test]
fn test_method_with_zero_zero_and_line_specific_mappings() {
    // Test case where a method has both 0:0: mappings and line-specific mappings.
    // The AndroidShadowContext class has method 'b' (obfuscated) with:
    // - 0:0: mapping pointing to line 68
    // - 1:4: mapping pointing to line 70
    // - 5:7: mapping pointing to line 71
    // etc.
    // When remapping a frame with line 3, it should match the 1:4: mapping (line 70),
    // NOT the 0:0: mapping (line 68), because we skip 0:0: mappings when line-specific
    // mappings exist.
    let mapper = ProguardMapper::new(ProguardMapping::new(MAPPING_ZERO_LINE_INFO));

    // Remap frame with method 'b' at line 3
    // This should match the 1:4: mapping (line 3 is in range 1-4) -> original line 70
    let mut mapped = mapper.remap_frame(&StackFrame::new("h2.a", "b", 3));

    let frame = mapped.next().unwrap();
    assert_eq!(
        frame.class(),
        "androidx.compose.ui.graphics.shadow.AndroidShadowContext"
    );
    assert_eq!(frame.method(), "obtainDropShadowRenderer-eZhPAX0");
    // Should map to line 70 (from the 1:4: mapping), not line 68 (from the 0:0: mapping)
    assert_eq!(frame.line(), 70);
    assert_eq!(mapped.next(), None);
}

#[test]
fn test_method_with_zero_zero_and_line_specific_mappings_cache() {
    // Test case where a method has both 0:0: mappings and line-specific mappings.
    // The AndroidShadowContext class has method 'b' (obfuscated) with:
    // - 0:0: mapping pointing to line 68
    // - 1:4: mapping pointing to line 70
    // - 5:7: mapping pointing to line 71
    // etc.
    // When remapping a frame with line 3, it should match the 1:4: mapping (line 70),
    // NOT the 0:0: mapping (line 68), because we skip 0:0: mappings when line-specific
    // mappings exist.
    let mapping = ProguardMapping::new(MAPPING_ZERO_LINE_INFO);
    let mut buf = Vec::new();
    ProguardCache::write(&mapping, &mut buf).unwrap();
    let cache = ProguardCache::parse(&buf).unwrap();
    cache.test();

    // Remap frame with method 'b' at line 3
    // This should match the 1:4: mapping (line 3 is in range 1-4) -> original line 70
    let frame = StackFrame::new("h2.a", "b", 3);
    let mut mapped = cache.remap_frame(&frame);

    let remapped_frame = mapped.next().unwrap();
    assert_eq!(
        remapped_frame.class(),
        "androidx.compose.ui.graphics.shadow.AndroidShadowContext"
    );
    assert_eq!(remapped_frame.method(), "obtainDropShadowRenderer-eZhPAX0");
    // Should map to line 70 (from the 1:4: mapping), not line 68 (from the 0:0: mapping)
    assert_eq!(remapped_frame.line(), 70);
    assert_eq!(mapped.next(), None);
}
