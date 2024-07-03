use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proguard::{ProguardCache, ProguardMapper, ProguardMapping};

static MAPPING: &[u8] = include_bytes!("../tests/res/mapping-inlines.txt");

static RAW: &str = r#"java.lang.RuntimeException: Button press caused an exception!
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

fn benchmark_remapping(c: &mut Criterion) {
    let mut cache_buf = Vec::new();
    let mapping = ProguardMapping::new(MAPPING);
    ProguardCache::write(&mapping, &mut cache_buf).unwrap();
    let cache = ProguardCache::parse(&cache_buf).unwrap();
    let mapper = ProguardMapper::new(mapping);

    let mut group = c.benchmark_group("Proguard Remapping");

    group.bench_function("Cache, preparsed", |b| {
        b.iter(|| cache.remap_stacktrace(black_box(RAW)))
    });
    group.bench_function("Mapper, preparsed", |b| {
        b.iter(|| mapper.remap_stacktrace(black_box(RAW)))
    });

    group.bench_function("Cache", |b| {
        b.iter(|| {
            let cache = ProguardCache::parse(black_box(&cache_buf)).unwrap();
            cache.remap_stacktrace(black_box(RAW))
        })
    });
    group.bench_function("Mapper", |b| {
        b.iter(|| {
            let mapper = ProguardMapper::new(black_box(ProguardMapping::new(MAPPING)));
            mapper.remap_stacktrace(black_box(RAW))
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_remapping);
criterion_main!(benches);
