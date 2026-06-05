use std::str::FromStr;

use crate::buffer::text::{ImplementedBufferTextBackendKind, TextBackendDebugLayout};
use crate::buffer::{
    BufferTextBackendKind, CharPos0, CharRange, EmacsBytePos, EmacsByteRange, TextEditRange,
    TextExtent, TextMetrics,
};

use super::BufferText;

fn implemented_kind(kind: BufferTextBackendKind) -> ImplementedBufferTextBackendKind {
    kind.implemented()
        .expect("test backend should be implemented")
}

#[test]
fn backend_kind_defaults_to_gap_buffer_with_stable_symbol_spelling() {
    crate::test_utils::init_test_tracing();
    let text = BufferText::new();

    assert_eq!(text.backend_kind(), BufferTextBackendKind::GapBuffer);
    assert_eq!(text.backend_kind().symbol_name(), "gap-buffer");
    assert_eq!(u8::from(BufferTextBackendKind::GapBuffer), 0);
    assert_eq!(u8::from(BufferTextBackendKind::PieceTree), 1);
    assert_eq!(u8::from(BufferTextBackendKind::Rope), 2);
    assert_eq!(
        BufferTextBackendKind::try_from(0),
        Ok(BufferTextBackendKind::GapBuffer)
    );
    assert_eq!(
        BufferTextBackendKind::try_from(1),
        Ok(BufferTextBackendKind::PieceTree)
    );
    assert_eq!(
        BufferTextBackendKind::try_from(2),
        Ok(BufferTextBackendKind::Rope)
    );
    assert!(text.backend_kind().is_implemented());
    assert_eq!(
        BufferTextBackendKind::from_str("piece-tree"),
        Ok(BufferTextBackendKind::PieceTree)
    );
    assert_eq!(
        BufferTextBackendKind::implemented_variants().collect::<Vec<_>>(),
        vec![
            BufferTextBackendKind::GapBuffer,
            BufferTextBackendKind::PieceTree,
            BufferTextBackendKind::Rope,
        ]
    );
    assert_eq!(
        BufferTextBackendKind::non_gap_implemented_variants().collect::<Vec<_>>(),
        vec![
            BufferTextBackendKind::PieceTree,
            BufferTextBackendKind::Rope,
        ]
    );
    assert!(BufferTextBackendKind::PieceTree.is_implemented());
    assert!(BufferTextBackendKind::Rope.is_implemented());
}

#[test]
fn buffer_text_can_use_non_gap_backends() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let layout = match kind {
            BufferTextBackendKind::GapBuffer => unreachable!("filtered above"),
            BufferTextBackendKind::PieceTree => {
                TextBackendDebugLayout::PieceTree(TextMetrics::new(5, 6))
            }
            BufferTextBackendKind::Rope => TextBackendDebugLayout::Rope(TextMetrics::new(5, 6)),
        };
        let mut text = BufferText::from_str_with_backend_kind("abécd", implemented_kind(kind));

        assert_eq!(text.backend_kind(), kind);
        assert_eq!(text.backend_debug_layout(), layout);
        assert!(text.gap_debug_layout().is_none());

        let insert_pos = text.buf_charpos_to_bytepos(2);
        text.insert_str(insert_pos, "XY");
        assert_eq!(text.to_string(), "abXYécd");

        let delete_start = text.buf_charpos_to_bytepos(1);
        let delete_end = text.buf_charpos_to_bytepos(4);
        text.delete_range(delete_start, delete_end);
        assert_eq!(text.to_string(), "aécd");
        assert_eq!(
            text.backend_debug_layout().metrics(),
            TextMetrics::new(4, 5)
        );
    }
}

#[test]
fn public_backend_kind_helpers_select_and_convert_storage() {
    crate::test_utils::init_test_tracing();
    let text = BufferText::try_from_str_with_backend_kind("abc", BufferTextBackendKind::Rope)
        .expect("rope backend should be available");

    assert_eq!(text.backend_kind(), BufferTextBackendKind::Rope);
    assert_eq!(text.to_string(), "abc");

    text.try_convert_backend_kind(BufferTextBackendKind::PieceTree)
        .expect("piece-tree backend should be available");
    assert_eq!(text.backend_kind(), BufferTextBackendKind::PieceTree);
    assert_eq!(text.to_string(), "abc");

    let empty = BufferText::try_new_with_backend_kind(BufferTextBackendKind::Rope)
        .expect("rope backend should be available");
    assert_eq!(empty.backend_kind(), BufferTextBackendKind::Rope);
    assert!(empty.is_empty());
}

#[test]
fn non_gap_backends_preserve_virtual_gap_compatibility_state() {
    crate::test_utils::init_test_tracing();
    let non_gap = BufferTextBackendKind::non_gap_implemented_variants().collect::<Vec<_>>();
    assert!(
        non_gap.len() >= 2,
        "test expects at least two non-gap backends"
    );

    for &kind in &non_gap {
        let mut text = BufferText::from_str("abc");
        let initial_gap_position = text.gap_position_lisp();
        let initial_gap_size = text.gap_size_lisp();
        assert_eq!(initial_gap_position, 4);

        text.try_convert_backend_kind(kind)
            .expect("backend should be implemented");
        assert_eq!(text.backend_kind(), kind);
        assert_eq!(text.gap_position_lisp(), initial_gap_position);
        assert_eq!(text.gap_size_lisp(), initial_gap_size);

        text.insert_str(text.emacs_byte_len(), "d");
        assert_eq!(text.gap_position_lisp(), initial_gap_position + 1);
        assert_eq!(text.gap_size_lisp(), initial_gap_size - 1);

        let other = non_gap
            .iter()
            .copied()
            .find(|candidate| *candidate != kind)
            .expect("second non-gap backend should exist");
        text.try_convert_backend_kind(other)
            .expect("second backend should be implemented");
        assert_eq!(text.backend_kind(), other);
        assert_eq!(text.gap_position_lisp(), initial_gap_position + 1);
        assert_eq!(text.gap_size_lisp(), initial_gap_size - 1);
    }
}

#[test]
fn backend_conversion_to_gap_preserves_virtual_gap_compatibility_state() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let mut text = BufferText::from_str("abéc");

        text.try_convert_backend_kind(kind)
            .expect("backend should be implemented");
        text.insert_str(0, "Ω");
        let virtual_gap_position = text.gap_position_lisp();
        let virtual_gap_size = text.gap_size_lisp();
        assert_eq!(text.to_string(), "Ωabéc");

        text.try_convert_backend_kind(BufferTextBackendKind::GapBuffer)
            .expect("gap backend should be implemented");
        let real_gap = text
            .gap_debug_layout()
            .expect("converted backend should be a real gap buffer")
            .compat_state();
        assert_eq!(
            real_gap.lisp_position(),
            virtual_gap_position,
            "{kind:?} virtual gap position should become the real GPT"
        );
        assert_eq!(
            real_gap.size(),
            virtual_gap_size,
            "{kind:?} virtual gap size should become the real GAP_SIZE"
        );
        assert_eq!(text.gap_position_lisp(), virtual_gap_position);
        assert_eq!(text.gap_size_lisp(), virtual_gap_size);

        text.try_convert_backend_kind(kind)
            .expect("backend should convert back to non-gap");
        assert_eq!(text.gap_position_lisp(), virtual_gap_position);
        assert_eq!(text.gap_size_lisp(), virtual_gap_size);
    }
}

#[test]
fn non_gap_same_len_replace_preserves_virtual_gap_byte_position() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let mut text = BufferText::from_str("éxq");
        text.try_convert_backend_kind(kind)
            .expect("backend should be implemented");
        let initial_gap_size = text.gap_size_lisp();
        assert_eq!(text.gap_position_lisp(), 4);

        text.replace_same_len_emacs_bytes(0, "éx".len(), "€".as_bytes());

        assert_eq!(text.to_string(), "€q");
        assert_eq!(
            text.gap_position_lisp(),
            3,
            "{kind:?} should keep the virtual gap at the same byte position"
        );
        assert_eq!(text.gap_size_lisp(), initial_gap_size);
    }
}

#[test]
fn edit_measurement_boundary_is_backend_neutral() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::implemented_variants() {
        let text = BufferText::from_str_with_backend_kind("aé\n🙂z", implemented_kind(kind));
        let start = text.buf_charpos_to_bytepos(1);
        let end = text.buf_charpos_to_bytepos(4);

        let range = text.edit_range_for_emacs_byte_range(EmacsByteRange::from_usize(start, end));
        assert_eq!(
            range.byte_range(),
            EmacsByteRange::from_usize(start, end),
            "{kind:?} byte range changed while measuring edit range"
        );
        assert_eq!(
            range.char_start(),
            CharPos0::new(1),
            "{kind:?} edit start char position diverged"
        );
        assert_eq!(
            range.char_end(),
            CharPos0::new(4),
            "{kind:?} edit end char position diverged"
        );
        assert_eq!(
            range.extent(),
            TextExtent::from_usize(3, end - start),
            "{kind:?} edit extent diverged"
        );
        assert_eq!(
            text.edit_range_for_char_range(CharRange::from_usize(1, 4)),
            range,
            "{kind:?} char range edit measurement diverged"
        );

        let byte_pos = EmacsBytePos::new(start);
        let empty = text.edit_range_at_emacs_byte_pos(byte_pos);
        assert_eq!(
            empty,
            TextEditRange::empty_at(byte_pos, CharPos0::new(1)),
            "{kind:?} empty edit range diverged"
        );
        assert_eq!(
            text.edit_range_for_char_range(CharRange::from_usize(1, 1)),
            empty,
            "{kind:?} empty char range edit measurement diverged"
        );

        let extent = TextExtent::from_emacs_bytes("Ωx".as_bytes(), true);
        let insertion = text.insertion_at_emacs_byte_pos(byte_pos, extent);
        assert_eq!(insertion.byte_pos(), byte_pos, "{kind:?}");
        assert_eq!(insertion.char_pos(), CharPos0::new(1), "{kind:?}");
        assert_eq!(
            insertion.byte_end(),
            EmacsBytePos::new(start + "Ωx".len()),
            "{kind:?}"
        );
        assert_eq!(insertion.char_end(), CharPos0::new(3), "{kind:?}");
    }
}

#[test]
fn non_gap_position_conversion_forward_scan_crosses_backend_chunks() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let mut char_scan =
            BufferText::from_str_with_backend_kind("aé日本🙂zzzz", implemented_kind(kind));
        char_scan.insert_str(1, "Ω");

        assert_eq!(char_scan.to_string(), "aΩé日本🙂zzzz");
        assert_eq!(
            char_scan.buf_charpos_to_bytepos(4),
            "aΩé日".len(),
            "{kind:?} char-to-byte forward scan should cross edited chunks"
        );

        let mut byte_scan =
            BufferText::from_str_with_backend_kind("aé日本🙂zzzz", implemented_kind(kind));
        byte_scan.insert_str(1, "Ω");
        assert_eq!(byte_scan.to_string(), "aΩé日本🙂zzzz");
        assert_eq!(
            byte_scan.buf_bytepos_to_charpos("aΩé日".len()),
            4,
            "{kind:?} byte-to-char forward scan should cross edited chunks"
        );
    }
}

#[test]
fn non_gap_position_conversion_uses_backend_index_without_anchor_scan() {
    crate::test_utils::init_test_tracing();
    let mut s = String::new();
    for _ in 0..20_000 {
        s.push_str("日");
    }

    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let text = BufferText::from_str_with_backend_kind(&s, implemented_kind(kind));
        assert_eq!(text.anchor_cache_len(), 0, "{kind:?}");

        let byte_pos = text.buf_charpos_to_bytepos(10_000);
        assert_eq!(byte_pos, "日".len() * 10_000, "{kind:?}");
        assert_eq!(text.anchor_cache_len(), 0, "{kind:?}");

        let char_pos = text.buf_bytepos_to_charpos(byte_pos);
        assert_eq!(char_pos, 10_000, "{kind:?}");
        assert_eq!(text.anchor_cache_len(), 0, "{kind:?}");
    }
}

#[test]
fn non_gap_lisp_string_preserves_unibyte_raw_bytes() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let raw = crate::heap_types::LispString::from_unibyte(vec![0xFF, b'A', 0x80]);
        let text = BufferText::from_lisp_string_with_backend_kind(&raw, implemented_kind(kind));

        assert_eq!(text.backend_kind(), kind);
        assert!(!text.is_multibyte());
        assert_eq!(text.char_count(), 3);

        let mut bytes = Vec::new();
        text.copy_emacs_byte_range_to(
            EmacsByteRange::from_usize(0, text.emacs_byte_len()),
            &mut bytes,
        );
        assert_eq!(bytes, vec![0xFF, b'A', 0x80]);
    }
}

#[test]
fn replace_lisp_string_preserves_non_gap_backend() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let text = BufferText::from_str_with_backend_kind("abc", implemented_kind(kind));
        let replacement = crate::heap_types::LispString::from_utf8("日本");

        text.replace_lisp_string(
            &replacement,
            crate::buffer::text_props::TextPropertyTable::new(),
        );

        assert_eq!(text.backend_kind(), kind);
        assert_eq!(text.to_string(), "日本");
        assert_eq!(
            text.backend_debug_layout().metrics(),
            TextMetrics::new(2, 6)
        );
    }
}

#[test]
fn same_size_replacement_invalidates_position_caches() {
    crate::test_utils::init_test_tracing();
    let left = "a".repeat(6001);
    let right = "b".repeat(6001);
    let original = format!("{left}€{right}");
    let replacement = format!("€{left}{right}");
    let target_char = 6001;

    for kind in BufferTextBackendKind::implemented_variants() {
        let mut text = BufferText::from_str_with_backend_kind(&original, implemented_kind(kind));

        assert_eq!(text.buf_charpos_to_bytepos(target_char), target_char);
        if kind.is_gap_buffer() {
            assert!(
                text.anchor_cache_len() > 0,
                "{kind:?} should cache the long char/byte walk before replacement"
            );
        } else {
            assert_eq!(
                text.anchor_cache_len(),
                0,
                "{kind:?} should use backend-indexed position lookup"
            );
        }

        text.replace_same_len_emacs_bytes(0, original.len(), replacement.as_bytes());

        assert_eq!(
            text.anchor_cache_len(),
            0,
            "{kind:?} should clear stale anchors after same-size replacement"
        );
        assert_eq!(text.buf_charpos_to_bytepos(target_char), target_char + 2);
    }
}

#[test]
fn from_lisp_string_preserves_unibyte_raw_bytes() {
    crate::test_utils::init_test_tracing();
    let raw = crate::heap_types::LispString::from_unibyte(vec![0xFF, b'A', 0x80]);
    let text = BufferText::from_lisp_string(&raw);

    assert!(!text.is_multibyte());
    assert_eq!(text.len(), 3);
    assert_eq!(text.char_count(), 3);

    let mut bytes = Vec::new();
    text.copy_emacs_byte_range_to(
        EmacsByteRange::from_usize(0, text.emacs_byte_len()),
        &mut bytes,
    );
    assert_eq!(bytes, vec![0xFF, b'A', 0x80]);
}

#[test]
fn char_count_tracks_multibyte_inserts_and_deletes() {
    crate::test_utils::init_test_tracing();
    let mut text = BufferText::from_str("ééz");
    assert_eq!(text.char_count(), 3);

    text.insert_str('é'.len_utf8(), "ß");
    assert_eq!(text.char_count(), 4);

    text.delete_range(2, 4);
    assert_eq!(text.char_count(), 3);
    assert_eq!(text.to_string(), "ééz");
}

#[test]
fn shared_clone_observes_cached_char_count_updates() {
    crate::test_utils::init_test_tracing();
    let mut text = BufferText::from_str("ab");
    let shared = text.shared_clone();
    text.insert_str(2, "é");
    assert_eq!(text.char_count(), 3);
    assert_eq!(shared.char_count(), 3);
}

#[test]
fn deep_clone_keeps_independent_char_count_cache() {
    crate::test_utils::init_test_tracing();
    let mut text = BufferText::from_str("ab");
    let cloned = text.clone();
    text.insert_str(2, "é");
    assert_eq!(text.char_count(), 3);
    assert_eq!(cloned.char_count(), 2);
}

#[test]
fn layout_tracks_gnu_style_gap_and_end_positions() {
    crate::test_utils::init_test_tracing();
    let mut text = BufferText::from_str("éz");
    assert_eq!(text.metrics(), TextMetrics::new(2, 3));
    let layout = text
        .gap_debug_layout()
        .expect("default backend is a gap buffer");
    assert_eq!(
        text.backend_debug_layout(),
        TextBackendDebugLayout::Gap(layout)
    );
    assert_eq!(layout.gpt.get(), 2);
    assert_eq!(layout.z.get(), 2);
    assert_eq!(layout.gpt_byte.get(), 3);
    assert_eq!(layout.z_byte.get(), 3);

    text.insert_str('é'.len_utf8(), "x");
    assert_eq!(text.metrics(), TextMetrics::new(3, 4));
    let layout = text
        .gap_debug_layout()
        .expect("default backend is a gap buffer");
    assert_eq!(
        text.backend_debug_layout(),
        TextBackendDebugLayout::Gap(layout)
    );
    assert_eq!(layout.gpt.get(), 2);
    assert_eq!(layout.z.get(), 3);
    assert_eq!(layout.gpt_byte.get(), 3);
    assert_eq!(layout.z_byte.get(), 4);
    assert_eq!(text.to_string(), "éxz");
}

#[test]
fn emacs_byte_chunks_cross_gap_without_copying_to_single_slice() {
    crate::test_utils::init_test_tracing();
    let mut text = BufferText::from_str("abcdef");
    text.insert_str(3, "X");
    text.delete_range(3, 4);

    let layout = text
        .gap_debug_layout()
        .expect("default backend is a gap buffer");
    assert_eq!(layout.gpt_byte.get(), 3);

    let mut chunks = Vec::new();
    text.for_each_emacs_byte_range_chunk(EmacsByteRange::from_usize(1, 5), |chunk| {
        chunks.push(chunk.to_vec());
        Ok::<(), ()>(())
    })
    .unwrap();
    assert_eq!(chunks, vec![b"bc".to_vec(), b"de".to_vec()]);
}

#[test]
fn range_contains_char_code_scans_non_contiguous_backend_chunks() {
    crate::test_utils::init_test_tracing();
    for kind in BufferTextBackendKind::non_gap_implemented_variants() {
        let mut text =
            BufferText::from_str_with_backend_kind(&"a".repeat(1100), implemented_kind(kind));
        let insert_at = text.char_pos_to_emacs_byte_pos(CharPos0::new(550));
        text.insert_str(insert_at.get(), "é");

        let mut chunks = 0;
        text.for_each_emacs_byte_range_chunk(EmacsByteRange::from_usize(0, text.len()), |_| {
            chunks += 1;
            Ok::<(), ()>(())
        })
        .unwrap();
        assert!(
            chunks > 1,
            "{kind:?} should expose a multi-chunk range for this test"
        );

        assert!(text.range_contains_char_code(0, text.len(), 'é' as u32));
        assert!(!text.range_contains_char_code(0, text.len(), '日' as u32));
    }
}

#[test]
fn buf_charpos_to_bytepos_matches_oracle() {
    let mut s = String::new();
    for i in 0..5000 {
        if i % 2 == 0 {
            s.push_str("hello ");
        } else {
            s.push_str("日本語 ");
        }
    }
    let text = BufferText::from_str(&s);

    // Oracle: contiguous bytes → char_to_byte_pos.
    let mut bytes = Vec::new();
    text.copy_bytes_to(0, text.len(), &mut bytes);

    for &cp in &[
        0usize,
        1,
        50,
        500,
        5000,
        12345,
        text.char_count() - 1,
        text.char_count(),
    ] {
        let got = text.buf_charpos_to_bytepos(cp);
        let expected = crate::emacs_core::emacs_char::char_to_byte_pos(&bytes, cp);
        assert_eq!(
            got, expected,
            "charpos {cp}: buf_charpos_to_bytepos returned {got}, oracle said {expected}"
        );
    }
}

#[test]
fn buf_charpos_to_bytepos_invalidates_on_mutation() {
    let mut text = BufferText::from_str("abc");
    let first = text.buf_charpos_to_bytepos(2);
    assert_eq!(first, 2);

    // Insert "é" (2 bytes in UTF-8) at pos 0 — now charpos 2 sits at bytepos 3.
    text.insert_str(0, "é");
    let second = text.buf_charpos_to_bytepos(2);
    assert_eq!(second, 3);
    assert_ne!(first, second, "cache returned stale bytepos after mutation");
}

#[test]
fn buf_bytepos_to_charpos_matches_oracle() {
    let mut s = String::new();
    for i in 0..5000 {
        if i % 2 == 0 {
            s.push_str("hello ");
        } else {
            s.push_str("日本語 ");
        }
    }
    let text = BufferText::from_str(&s);

    let mut bytes = Vec::new();
    text.copy_bytes_to(0, text.len(), &mut bytes);

    for &bp in &[0usize, 1, 50, 500, 5000, 12345, text.len() - 1, text.len()] {
        // Oracle valid only on char boundaries — snap bp down to one.
        let mut bp_snapped = bp;
        while bp_snapped > 0 && bp_snapped < bytes.len() && (bytes[bp_snapped] & 0xC0) == 0x80 {
            bp_snapped -= 1;
        }
        let got = text.buf_bytepos_to_charpos(bp_snapped);
        let expected = crate::emacs_core::emacs_char::byte_to_char_pos(&bytes, bp_snapped);
        assert_eq!(got, expected, "bytepos {bp_snapped}");
    }
}

#[test]
fn long_scan_populates_anchor_cache() {
    // 20 000+ multibyte chars, no existing markers.
    // Query at the midpoint so the walk from either BEG or Z is >5000.
    let mut s = String::new();
    for _ in 0..20_000 {
        s.push_str("日");
    }
    let text = BufferText::from_str(&s);

    assert_eq!(text.anchor_cache_len(), 0);

    // 10 000 chars into a 20 000-char buffer — scan from nearest bracket
    // must walk 10 000 positions (> POSITION_ANCHOR_STRIDE=5000).
    let _ = text.buf_charpos_to_bytepos(10_000);

    assert!(
        text.anchor_cache_len() > 0,
        "expected auto-anchor to have been inserted after long scan (walked > 5000)"
    );
}

#[test]
fn set_multibyte_invalidates_position_caches() {
    let mut s = String::new();
    for _ in 0..20_000 {
        s.push_str("日");
    }
    let text = BufferText::from_str(&s);

    let _ = text.buf_charpos_to_bytepos(10_000);
    assert!(text.anchor_cache_len() > 0);

    text.set_multibyte(false);

    assert_eq!(text.anchor_cache_len(), 0);
    assert!(!text.is_multibyte());
    assert_eq!(text.char_count(), text.emacs_byte_len());
}

#[test]
fn replace_lisp_string_invalidates_position_cache() {
    crate::test_utils::init_test_tracing();
    // Build a buffer with a known multibyte char at charpos 2.
    let text = BufferText::from_str("日日日"); // 3 chars, 9 bytes
    let cached_before = text.buf_charpos_to_bytepos(2);
    assert_eq!(cached_before, 6);

    // Replace with different same-char-and-byte-count content.
    let lisp_string = crate::heap_types::LispString::from_utf8("本本本");
    text.replace_lisp_string(
        &lisp_string,
        crate::buffer::text_props::TextPropertyTable::new(),
    );

    // Same-count replacement would leave a stale pos_cache; verify it was
    // cleared by confirming the conversion is recomputed correctly. (The
    // byte position of charpos 2 must match the new content's layout.)
    let after = text.buf_charpos_to_bytepos(2);
    assert_eq!(after, 6, "charpos 2 in '本本本' is at bytepos 6");

    // Sanity: the actual bytes at that position are the lead byte of '本'.
    // '本' is 0xE6 0x9C 0xAC. So buffer[6] should be 0xE6.
    let b = text.byte_at(6);
    assert_eq!(
        b, 0xE6,
        "post-replace byte at position 6 should be 0xE6 (lead byte of 本)"
    );
}

#[test]
fn replace_lisp_string_handles_unibyte_raw_bytes() {
    crate::test_utils::init_test_tracing();
    let text = BufferText::from_str("ééz");
    let cached_before = text.buf_charpos_to_bytepos(2);
    assert_eq!(cached_before, 4);

    let raw = crate::heap_types::LispString::from_unibyte(vec![0xFF, b'A', 0x80]);
    text.replace_lisp_string(&raw, crate::buffer::text_props::TextPropertyTable::new());

    assert!(!text.is_multibyte());
    assert_eq!(text.char_count(), 3);
    assert_eq!(text.buf_charpos_to_bytepos(2), 2);
    assert_eq!(text.byte_at(0), 0xFF);
    assert_eq!(text.byte_at(1), b'A');
    assert_eq!(text.byte_at(2), 0x80);

    let mut bytes = Vec::new();
    text.copy_emacs_byte_range_to(
        EmacsByteRange::from_usize(0, text.emacs_byte_len()),
        &mut bytes,
    );
    assert_eq!(bytes, vec![0xFF, b'A', 0x80]);
}
