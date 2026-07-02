use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use neomacs_display_protocol::face::Face;
use neomacs_display_protocol::glyph_matrix::{
    FrameDisplayState, Glyph, GlyphArea, GlyphMatrix, WindowMatrixEntry,
};
use neomacs_display_protocol::types::Rect;

fn frame_state(cols: usize, rows: usize) -> FrameDisplayState {
    let char_w = 8.0;
    let char_h = 16.0;
    let mut state = FrameDisplayState::new(cols, rows, char_w, char_h);
    state.faces.insert(0, Face::new(0));

    let mut matrix = GlyphMatrix::new(rows, cols);
    for row_idx in 0..rows {
        let row = &mut matrix.rows[row_idx];
        row.enabled = true;
        for col in 0..cols {
            let ch = (b'a' + (col % 26) as u8) as char;
            row.glyphs[GlyphArea::Text as usize].push(Glyph::char(ch, 0, col));
        }
        row.hash = row.compute_hash();
    }

    state.window_matrices.push(WindowMatrixEntry {
        window_id: 1,
        matrix,
        pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, rows as f32 * char_h),
        text_pixel_bounds: Rect::new(0.0, 0.0, cols as f32 * char_w, rows as f32 * char_h),
        selected: true,
    });
    state
}

fn bench_materialize(c: &mut Criterion) {
    c.bench_function("materialize_120x40", |b| {
        let state = frame_state(120, 40);
        b.iter_batched(
            || state.clone(),
            |state| black_box(state.materialize()),
            BatchSize::SmallInput,
        );
    });

    c.bench_function("materialize_200x60", |b| {
        let state = frame_state(200, 60);
        b.iter_batched(
            || state.clone(),
            |state| black_box(state.materialize()),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_materialize);
criterion_main!(benches);
