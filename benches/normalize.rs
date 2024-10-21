use arabic_normalize::Normalizer;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn bench(c: &mut Criterion) {
    let vocalized = "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ ٱلْحَمْدُ لِلَّهِ رَبِّ ٱلْعَٰلَمِينَ ".repeat(200);
    let clean = "بسم الله الرحمن الرحيم الحمد لله رب العالمين ".repeat(200);
    let ascii = "The quick brown fox jumps over the lazy dog 0123456789 ".repeat(200);
    let n = Normalizer::search();
    let mut g = c.benchmark_group("normalize");
    for (name, text) in [("vocalized", &vocalized), ("already_clean", &clean), ("ascii", &ascii)] {
        g.throughput(Throughput::Bytes(text.len() as u64));
        g.bench_function(name, |b| b.iter(|| n.normalize(black_box(text)).len()));
    }
    g.throughput(Throughput::Bytes(vocalized.len() as u64));
    g.bench_function("search_terms", |b| b.iter(|| n.search_terms(black_box(&vocalized)).len()));
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
