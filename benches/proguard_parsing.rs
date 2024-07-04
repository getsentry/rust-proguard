use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proguard::{ProguardCache, ProguardMapper, ProguardMapping};

static MAPPING: &[u8] = include_bytes!("../tests/res/mapping.txt");

fn proguard_mapper(mapping: ProguardMapping) -> ProguardMapper {
    ProguardMapper::new(mapping)
}

fn proguard_cache(cache: &[u8]) -> ProguardCache {
    ProguardCache::parse(cache).unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut cache = Vec::new();
    let mapping = ProguardMapping::new(MAPPING);
    ProguardCache::write(&mapping, &mut cache).unwrap();

    let mut group = c.benchmark_group("Proguard Parsing");
    group.bench_function("Proguard Mapper", |b| {
        b.iter(|| proguard_mapper(black_box(mapping.clone())))
    });

    group.bench_function("Proguard Cache creation", |b| {
        b.iter(|| {
            let mut cache = Vec::new();
            let mapping = ProguardMapping::new(MAPPING);
            ProguardCache::write(&mapping, &mut cache).unwrap();
        })
    });

    group.bench_function("Proguard Cache parsing", |b| {
        b.iter(|| proguard_cache(black_box(&cache)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(25);
    targets = criterion_benchmark
}
criterion_main!(benches);
