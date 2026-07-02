use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use neomacs_layout_engine::font_metrics::FontMetricsService;

fn bench_char_width(c: &mut Criterion) {
    let mut metrics = FontMetricsService::new();
    let family = "monospace";
    let weight = 400;
    let italic = false;
    let font_size = 16.0;

    c.bench_function("char_width_ascii_hot_loop", |b| {
        let text = "fn main() { println!(\"hello\"); }";
        b.iter(|| {
            let mut total = 0.0;
            for ch in text.chars() {
                total += metrics.char_width(black_box(ch), family, weight, italic, font_size);
            }
            black_box(total)
        });
    });

    c.bench_function("char_width_mixed_unicode_hot_loop", |b| {
        let text = "ASCII λ 中 😀 عربى";
        b.iter(|| {
            let mut total = 0.0;
            for ch in text.chars() {
                total += metrics.char_width(black_box(ch), family, weight, italic, font_size);
            }
            black_box(total)
        });
    });
}

criterion_group!(benches, bench_char_width);
criterion_main!(benches);
