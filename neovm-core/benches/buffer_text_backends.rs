//! Buffer text backend benchmarks.
//!
//! Runs with:
//!
//! cargo bench -p neovm-core --bench buffer_text_backends

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use neovm_core::buffer::{BufferText, BufferTextBackendKind};

const BACKENDS: [BufferTextBackendKind; 3] = [
    BufferTextBackendKind::GapBuffer,
    BufferTextBackendKind::PieceTree,
    BufferTextBackendKind::Rope,
];

const EDITS_PER_ITER: usize = 512;
const COPY_RANGES_PER_ITER: usize = 1024;
const CONVERSIONS_PER_ITER: usize = 4096;

fn sample_text(lines: usize) -> String {
    let mut out = String::with_capacity(lines * 96);
    for line in 0..lines {
        out.push_str("line ");
        out.push_str(&(line % 100_000).to_string());
        out.push_str(": alpha beta gamma delta ");
        match line % 5 {
            0 => out.push_str("\u{03bb}\u{65e5}\u{672c}\u{1f642}"),
            1 => out.push_str("cafe\u{0301} resume\u{0301}"),
            2 => out.push_str("\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}"),
            3 => {
                out.push_str("\u{0939}\u{093f}\u{0928}\u{094d}\u{0926}\u{0940}");
            }
            _ => out.push_str("plain ascii words"),
        }
        out.push('\n');
    }
    out
}

fn text_with_backend(text: &str, kind: BufferTextBackendKind) -> BufferText {
    BufferText::try_from_str_with_backend_kind(text, kind).expect("backend should be implemented")
}

fn byte_ranges_for_char_windows(
    text: &BufferText,
    count: usize,
    window_chars: usize,
) -> Vec<(usize, usize)> {
    let total_chars = text.char_count();
    let max_start = total_chars.saturating_sub(window_chars);
    let mut ranges = Vec::with_capacity(count);
    for index in 0..count {
        let start_char = if max_start == 0 {
            0
        } else {
            (index * 7_919) % max_start
        };
        let end_char = (start_char + window_chars).min(total_chars);
        ranges.push((
            text.buf_charpos_to_bytepos(start_char),
            text.buf_charpos_to_bytepos(end_char),
        ));
    }
    ranges
}

fn scattered_char_positions(text: &BufferText, count: usize) -> Vec<usize> {
    let total_chars = text.char_count();
    let mut positions = Vec::with_capacity(count);
    for index in 0..count {
        positions.push(if total_chars == 0 {
            0
        } else {
            (index * 10_007) % total_chars
        });
    }
    positions
}

fn bench_construct_large(c: &mut Criterion) {
    let input = sample_text(16_384);
    let mut group = c.benchmark_group("buffer_text_backend/construct_large");
    group.throughput(Throughput::Bytes(input.len() as u64));
    for kind in BACKENDS {
        group.bench_function(BenchmarkId::from_parameter(kind.symbol_name()), |b| {
            b.iter(|| black_box(text_with_backend(black_box(input.as_str()), kind)));
        });
    }
    group.finish();
}

fn bench_scattered_edit_churn(c: &mut Criterion) {
    let input = sample_text(4_096);
    let inserts = ["x", "\u{03bb}", "\u{65e5}\u{672c}", "abc", "\u{1f642}"];
    let mut group = c.benchmark_group("buffer_text_backend/scattered_edit_churn");
    group.throughput(Throughput::Elements(EDITS_PER_ITER as u64));
    for kind in BACKENDS {
        group.bench_function(BenchmarkId::from_parameter(kind.symbol_name()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let mut text = text_with_backend(&input, kind);
                    let start = Instant::now();
                    for edit in 0..EDITS_PER_ITER {
                        let chars = text.char_count();
                        let char_pos = (edit * 9_973) % (chars + 1);
                        let byte_pos = text.buf_charpos_to_bytepos(char_pos);
                        let inserted = inserts[edit % inserts.len()];
                        text.insert_str(byte_pos, inserted);
                        let end_pos =
                            text.buf_charpos_to_bytepos(char_pos + inserted.chars().count());
                        text.delete_range(byte_pos, end_pos);
                    }
                    black_box(text.len());
                    total += start.elapsed();
                }
                total
            });
        });
    }
    group.finish();
}

fn bench_copy_ranges(c: &mut Criterion) {
    let input = sample_text(8_192);
    let mut group = c.benchmark_group("buffer_text_backend/copy_ranges");
    group.throughput(Throughput::Elements(COPY_RANGES_PER_ITER as u64));
    for kind in BACKENDS {
        let text = text_with_backend(&input, kind);
        let ranges = byte_ranges_for_char_windows(&text, COPY_RANGES_PER_ITER, 96);
        group.bench_function(BenchmarkId::from_parameter(kind.symbol_name()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                let mut out = Vec::with_capacity(1024);
                for _ in 0..iters {
                    let start = Instant::now();
                    for &(start_byte, end_byte) in &ranges {
                        out.clear();
                        text.copy_bytes_to(start_byte, end_byte, &mut out);
                        black_box(out.as_slice());
                    }
                    total += start.elapsed();
                }
                total
            });
        });
    }
    group.finish();
}

fn bench_char_to_byte(c: &mut Criterion) {
    let input = sample_text(8_192);
    let mut group = c.benchmark_group("buffer_text_backend/char_to_byte");
    group.throughput(Throughput::Elements(CONVERSIONS_PER_ITER as u64));
    for kind in BACKENDS {
        let text = text_with_backend(&input, kind);
        let positions = scattered_char_positions(&text, CONVERSIONS_PER_ITER);
        group.bench_function(BenchmarkId::from_parameter(kind.symbol_name()), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    let text = text_with_backend(&input, kind);
                    let start = Instant::now();
                    let mut checksum = 0usize;
                    for &position in &positions {
                        checksum ^= text.buf_charpos_to_bytepos(position);
                    }
                    black_box(checksum);
                    total += start.elapsed();
                }
                total
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_construct_large,
    bench_scattered_edit_churn,
    bench_copy_ranges,
    bench_char_to_byte,
);
criterion_main!(benches);
