use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use neomacs_layout_engine::LayoutEngine;
use neovm_core::buffer::{EmacsBytePos, EmacsByteRange};
use neovm_core::emacs_core::{Context, Value};
use neovm_core::window::FrameId;

fn mixed_face_fixture() -> (Context, LayoutEngine, FrameId) {
    let mut eval = Context::new();
    let buf_id = eval
        .buffer_manager()
        .current_buffer()
        .expect("current buffer")
        .id();

    let mut text = String::with_capacity(10_000 * 48);
    let mut face_ranges = Vec::new();
    for line in 0..10_000 {
        let start = text.len();
        if line % 3 == 0 {
            text.push_str("(defun sample-fn (alpha beta) (+ alpha beta))");
        } else if line % 3 == 1 {
            text.push_str("let total = alpha + beta + gamma; // mixed face row");
        } else {
            text.push_str("plain text row with unicode lambda lambda");
        }
        let end = text.len();
        text.push('\n');
        if line % 2 == 0 {
            face_ranges.push((start, end));
        }
    }
    eval.buffer_manager_mut()
        .insert_into_buffer(buf_id, &text)
        .expect("insert benchmark text");

    {
        let buffer = eval.buffer_manager_mut().get_mut(buf_id).expect("buffer");
        for (start, end) in face_ranges {
            buffer.text_props_put_property_in_emacs_byte_range(
                EmacsByteRange::new(EmacsBytePos::new(start), EmacsBytePos::new(end)),
                Value::symbol("face"),
                Value::symbol("font-lock-keyword-face"),
            );
        }
    }

    let frame_id =
        eval.frame_manager_mut()
            .create_frame("layout-bench-10k-mixed-faces", 1000, 700, buf_id);
    {
        let frame = eval.frame_manager_mut().get_mut(frame_id).expect("frame");
        frame.set_window_system(Some(Value::symbol("neo")));
        frame.install_gnu_gui_default_parameters();
    }
    assert!(eval.frame_manager_mut().select_frame(frame_id));

    let mut engine = LayoutEngine::new();
    engine.layout_frame_rust(&mut eval, frame_id);
    (eval, engine, frame_id)
}

fn bench_layout_window(c: &mut Criterion) {
    c.bench_function("layout_frame_rust_10k_line_mixed_faces", |b| {
        let (mut eval, mut engine, frame_id) = mixed_face_fixture();
        b.iter(|| {
            engine.layout_frame_rust(&mut eval, frame_id);
            black_box(frame_id);
        });
    });
}

criterion_group!(benches, bench_layout_window);
criterion_main!(benches);
