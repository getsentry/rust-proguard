use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proguard::{ProguardMapper, ProguardMapping};

static MAPPING: &[u8] = include_bytes!("../tests/res/mapping.txt");

fn proguard_mapper(mapping: ProguardMapping) -> ProguardMapper {
    ProguardMapper::new(mapping)
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("proguard mapper", |b| {
        b.iter(|| proguard_mapper(black_box(ProguardMapping::new(MAPPING))))
    });
}

criterion_group!{
    name = benches;
    config = Criterion::default().sample_size(25);
    targets = criterion_benchmark
}
criterion_main!(benches);
