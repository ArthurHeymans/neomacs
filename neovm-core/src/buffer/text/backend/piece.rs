use std::fmt;

use crate::buffer::buffer_text::{BufferTextBackendLayout, BufferTextMetrics};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PieceSource {
    Original,
    Add,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    source: PieceSource,
    start: usize,
    len: usize,
    chars: usize,
}

#[derive(Clone)]
struct PieceNode {
    piece: Piece,
    priority: u64,
    metrics: BufferTextMetrics,
    left: Option<Box<PieceNode>>,
    right: Option<Box<PieceNode>>,
}

impl PieceNode {
    fn new(piece: Piece, priority: u64) -> Box<Self> {
        Box::new(Self {
            piece,
            priority,
            metrics: piece_metrics(piece),
            left: None,
            right: None,
        })
    }

    fn refresh(&mut self) {
        let left = node_metrics(&self.left);
        let right = node_metrics(&self.right);
        self.metrics = BufferTextMetrics {
            z: left.z + self.piece.chars + right.z,
            z_byte: left.z_byte + self.piece.len + right.z_byte,
        };
    }
}

#[derive(Clone)]
pub(in crate::buffer) struct PieceTreeTextBackend {
    original: Vec<u8>,
    add: Vec<u8>,
    multibyte: bool,
    root: Option<Box<PieceNode>>,
    next_piece_id: u64,
}

impl PieceTreeTextBackend {
    pub(in crate::buffer) fn new() -> Self {
        Self {
            original: Vec::new(),
            add: Vec::new(),
            multibyte: true,
            root: None,
            next_piece_id: 1,
        }
    }

    pub(in crate::buffer) fn from_str(text: &str) -> Self {
        let multibyte = !text.chars().any(|ch| {
            let code = ch as u32;
            (0xE300..=0xE3FF).contains(&code)
        });
        let bytes =
            crate::emacs_core::string_escape::storage_string_to_buffer_bytes(text, multibyte);
        Self::from_emacs_bytes(&bytes, multibyte)
    }

    pub(in crate::buffer) fn from_emacs_bytes(bytes: &[u8], multibyte: bool) -> Self {
        let mut backend = Self {
            original: bytes.to_vec(),
            add: Vec::new(),
            multibyte,
            root: None,
            next_piece_id: 1,
        };
        backend.root = backend.node_for_piece(Piece {
            source: PieceSource::Original,
            start: 0,
            len: bytes.len(),
            chars: emacs_char_count_bytes(bytes, multibyte),
        });
        backend
    }

    pub(in crate::buffer) fn from_dump(text: Vec<u8>, multibyte: bool) -> Self {
        Self::from_emacs_bytes(&text, multibyte)
    }

    pub(in crate::buffer) fn kind(&self) -> crate::buffer::BufferTextBackendKind {
        crate::buffer::BufferTextBackendKind::PieceTree
    }

    pub(in crate::buffer) fn layout(&self) -> BufferTextBackendLayout {
        BufferTextBackendLayout::PieceTree(self.metrics())
    }

    pub(in crate::buffer) fn len(&self) -> usize {
        self.metrics().z_byte
    }

    pub(in crate::buffer) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(in crate::buffer) fn is_multibyte(&self) -> bool {
        self.multibyte
    }

    pub(in crate::buffer) fn set_multibyte(&mut self, multibyte: bool) {
        if self.multibyte == multibyte {
            return;
        }
        let bytes = self.dump_text();
        self.rebuild_from_bytes(bytes, multibyte);
    }

    pub(in crate::buffer) fn byte_at(&self, pos: usize) -> u8 {
        assert!(
            pos < self.len(),
            "byte_at: position {pos} out of range (len {})",
            self.len()
        );
        self.contiguous_slice(pos, pos + 1).expect("single byte")[0]
    }

    pub(in crate::buffer) fn emacs_byte_at(&self, pos: usize) -> Option<u8> {
        (pos < self.len()).then(|| self.byte_at(pos))
    }

    pub(in crate::buffer) fn char_at(&self, pos: usize) -> Option<char> {
        self.char_code_at(pos).and_then(char::from_u32)
    }

    pub(in crate::buffer) fn char_code_at(&self, pos: usize) -> Option<u32> {
        if pos >= self.len() {
            return None;
        }
        self.byte_to_char(pos);
        if !self.multibyte {
            return Some(self.byte_at(pos) as u32);
        }

        let mut tmp = [0u8; crate::emacs_core::emacs_char::MAX_MULTIBYTE_LENGTH];
        let available = (self.len() - pos).min(tmp.len());
        let mut written = 0;
        self.for_each_emacs_byte_chunk(pos, pos + available, |chunk| {
            let take = (available - written).min(chunk.len());
            tmp[written..written + take].copy_from_slice(&chunk[..take]);
            written += take;
            Ok::<(), ()>(())
        })
        .expect("infallible chunk copy");
        Some(crate::emacs_core::emacs_char::string_char(&tmp[..written]).0)
    }

    pub(in crate::buffer) fn byte_to_char(&self, byte_pos: usize) -> usize {
        assert!(
            byte_pos <= self.len(),
            "byte_to_char: byte_pos ({byte_pos}) > len ({})",
            self.len()
        );
        self.byte_to_char_in_node(&self.root, byte_pos)
    }

    pub(in crate::buffer) fn char_to_byte(&self, char_pos: usize) -> usize {
        let metrics = self.metrics();
        if char_pos >= metrics.z {
            if char_pos > metrics.z {
                tracing::debug!(
                    "piece tree char_to_byte: char_pos ({char_pos}) exceeds char_count ({}), clamping",
                    metrics.z
                );
            }
            return metrics.z_byte;
        }
        self.char_to_byte_in_node(&self.root, char_pos)
    }

    pub(in crate::buffer) fn storage_byte_to_emacs_byte(&self, byte_pos: usize) -> usize {
        byte_pos.min(self.len())
    }

    pub(in crate::buffer) fn emacs_byte_to_storage_byte(&self, byte_pos: usize) -> usize {
        byte_pos.min(self.len())
    }

    pub(in crate::buffer) fn text_range(&self, start: usize, end: usize) -> String {
        assert!(start <= end, "text_range: start ({start}) > end ({end})");
        assert!(
            end <= self.len(),
            "text_range: end ({end}) > len ({})",
            self.len()
        );
        let mut out = Vec::with_capacity(end - start);
        self.copy_bytes_to(start, end, &mut out);
        crate::emacs_core::string_escape::emacs_bytes_to_storage_string(&out, self.multibyte)
    }

    pub(in crate::buffer) fn copy_bytes_to(&self, start: usize, end: usize, out: &mut Vec<u8>) {
        assert!(start <= end, "copy_bytes_to: start ({start}) > end ({end})");
        assert!(
            end <= self.len(),
            "copy_bytes_to: end ({end}) > len ({})",
            self.len()
        );
        out.clear();
        out.reserve(end - start);
        self.for_each_emacs_byte_chunk(start, end, |chunk| {
            out.extend_from_slice(chunk);
            Ok::<(), ()>(())
        })
        .expect("infallible byte copy");
    }

    pub(in crate::buffer) fn copy_emacs_bytes_to(
        &self,
        start: usize,
        end: usize,
        out: &mut Vec<u8>,
    ) {
        assert!(
            start <= end,
            "copy_emacs_bytes_to: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "copy_emacs_bytes_to: end ({end}) > emacs len ({})",
            self.len()
        );
        self.copy_bytes_to(start, end, out);
    }

    pub(in crate::buffer) fn for_each_emacs_byte_chunk<E>(
        &self,
        start: usize,
        end: usize,
        mut f: impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        assert!(
            start <= end,
            "for_each_emacs_byte_chunk: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "for_each_emacs_byte_chunk: end ({end}) > emacs len ({})",
            self.len()
        );
        self.for_each_range(&self.root, start, end, &mut f)
    }

    pub(in crate::buffer) fn has_contiguous_emacs_bytes(&self, start: usize, end: usize) -> bool {
        assert!(
            start <= end,
            "has_contiguous_emacs_bytes: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "has_contiguous_emacs_bytes: end ({end}) > emacs len ({})",
            self.len()
        );
        start == end || self.contiguous_slice(start, end).is_some()
    }

    pub(in crate::buffer) fn with_contiguous_emacs_bytes<R>(
        &self,
        start: usize,
        end: usize,
        f: impl FnOnce(&[u8]) -> R,
    ) -> Option<R> {
        assert!(
            start <= end,
            "with_contiguous_emacs_bytes: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "with_contiguous_emacs_bytes: end ({end}) > emacs len ({})",
            self.len()
        );
        if start == end {
            return Some(f(&[]));
        }
        self.contiguous_slice(start, end).map(f)
    }

    pub(in crate::buffer) fn insert_str(&mut self, pos: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let bytes =
            crate::emacs_core::string_escape::storage_string_to_buffer_bytes(text, self.multibyte);
        self.insert_emacs_bytes(pos, &bytes);
    }

    pub(in crate::buffer) fn insert_emacs_bytes(&mut self, pos: usize, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let nchars = emacs_char_count_bytes(bytes, self.multibyte);
        self.insert_emacs_bytes_both(pos, bytes, nchars);
    }

    pub(in crate::buffer) fn insert_emacs_bytes_both(
        &mut self,
        pos: usize,
        bytes: &[u8],
        nchars: usize,
    ) {
        assert!(
            pos <= self.len(),
            "insert_emacs_bytes_both: position {pos} out of range (len {})",
            self.len()
        );
        if bytes.is_empty() {
            return;
        }
        debug_assert_eq!(
            nchars,
            emacs_char_count_bytes(bytes, self.multibyte),
            "insert_emacs_bytes_both: caller-supplied nchars mismatches actual"
        );
        self.byte_to_char(pos);

        let add_start = self.add.len();
        self.add.extend_from_slice(bytes);
        let piece = self.node_for_piece(Piece {
            source: PieceSource::Add,
            start: add_start,
            len: bytes.len(),
            chars: nchars,
        });
        let root = self.root.take();
        let (left, right) = self.split_at_byte(root, pos);
        self.root = Self::merge(Self::merge(left, piece), right);
    }

    pub(in crate::buffer) fn delete_range(&mut self, start: usize, end: usize) {
        assert!(start <= end, "delete_range: start ({start}) > end ({end})");
        assert!(
            end <= self.len(),
            "delete_range: end ({end}) > len ({})",
            self.len()
        );
        if start == end {
            return;
        }
        let nchars = self.byte_to_char(end) - self.byte_to_char(start);
        self.delete_range_both(start, end, nchars);
    }

    pub(in crate::buffer) fn delete_range_both(&mut self, start: usize, end: usize, nchars: usize) {
        assert!(
            start <= end,
            "delete_range_both: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "delete_range_both: end ({end}) > len ({})",
            self.len()
        );
        if start == end {
            return;
        }
        debug_assert_eq!(
            nchars,
            self.byte_to_char(end) - self.byte_to_char(start),
            "delete_range_both: caller-supplied nchars mismatches actual"
        );

        let root = self.root.take();
        let (left, rest) = self.split_at_byte(root, start);
        let (_deleted, right) = self.split_at_byte(rest, end - start);
        self.root = Self::merge(left, right);
    }

    pub(in crate::buffer) fn replace_same_len_emacs_bytes(
        &mut self,
        start: usize,
        end: usize,
        replacement: &[u8],
    ) {
        assert!(
            start <= end,
            "replace_same_len_range: start ({start}) > end ({end})"
        );
        assert!(
            end <= self.len(),
            "replace_same_len_range: end ({end}) > len ({})",
            self.len()
        );
        assert_eq!(
            replacement.len(),
            end - start,
            "replace_same_len_range: replacement Emacs-byte length ({}) must match replaced length ({})",
            replacement.len(),
            end - start
        );
        if start == end {
            return;
        }

        let old_chars = self.byte_to_char(end) - self.byte_to_char(start);
        self.delete_range_both(start, end, old_chars);
        let new_chars = emacs_char_count_bytes(replacement, self.multibyte);
        self.insert_emacs_bytes_both(start, replacement, new_chars);
    }

    pub(in crate::buffer) fn dump_text(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len());
        self.copy_emacs_bytes_to(0, self.len(), &mut out);
        out
    }

    fn metrics(&self) -> BufferTextMetrics {
        node_metrics(&self.root)
    }

    fn rebuild_from_bytes(&mut self, bytes: Vec<u8>, multibyte: bool) {
        self.original = bytes;
        self.add.clear();
        self.multibyte = multibyte;
        self.root = None;
        self.next_piece_id = 1;
        self.root = self.node_for_piece(Piece {
            source: PieceSource::Original,
            start: 0,
            len: self.original.len(),
            chars: emacs_char_count_bytes(&self.original, multibyte),
        });
    }

    fn next_priority(&mut self) -> u64 {
        let id = self.next_piece_id;
        self.next_piece_id = self.next_piece_id.wrapping_add(1);
        splitmix64(id)
    }

    fn node_for_piece(&mut self, piece: Piece) -> Option<Box<PieceNode>> {
        (piece.len > 0).then(|| PieceNode::new(piece, self.next_priority()))
    }

    fn split_at_byte(
        &mut self,
        tree: Option<Box<PieceNode>>,
        byte_pos: usize,
    ) -> (Option<Box<PieceNode>>, Option<Box<PieceNode>>) {
        let Some(mut node) = tree else {
            assert_eq!(byte_pos, 0, "split_at_byte: byte_pos out of empty tree");
            return (None, None);
        };

        assert!(
            byte_pos <= node.metrics.z_byte,
            "split_at_byte: byte_pos ({byte_pos}) > subtree len ({})",
            node.metrics.z_byte
        );

        let left_metrics = node_metrics(&node.left);
        let piece_start = left_metrics.z_byte;
        let piece_end = piece_start + node.piece.len;

        if byte_pos < piece_start {
            let (left, right_of_left) = self.split_at_byte(node.left.take(), byte_pos);
            node.left = right_of_left;
            node.refresh();
            return (left, Some(node));
        }

        if byte_pos > piece_end {
            let (left_of_right, right) =
                self.split_at_byte(node.right.take(), byte_pos - piece_end);
            node.right = left_of_right;
            node.refresh();
            return (Some(node), right);
        }

        let local = byte_pos - piece_start;
        if local == 0 {
            let left = node.left.take();
            node.refresh();
            return (left, Some(node));
        }
        if local == node.piece.len {
            let right = node.right.take();
            node.refresh();
            return (Some(node), right);
        }

        let (left_piece, right_piece) = self.split_piece(node.piece, local);
        let left_tree = Self::merge(node.left.take(), self.node_for_piece(left_piece));
        let right_tree = Self::merge(self.node_for_piece(right_piece), node.right.take());
        (left_tree, right_tree)
    }

    fn split_piece(&self, piece: Piece, local_byte: usize) -> (Piece, Piece) {
        debug_assert!(local_byte > 0 && local_byte < piece.len);
        let chars_before = self.piece_byte_to_char(piece, local_byte);
        (
            Piece {
                source: piece.source,
                start: piece.start,
                len: local_byte,
                chars: chars_before,
            },
            Piece {
                source: piece.source,
                start: piece.start + local_byte,
                len: piece.len - local_byte,
                chars: piece.chars - chars_before,
            },
        )
    }

    fn merge(
        left: Option<Box<PieceNode>>,
        right: Option<Box<PieceNode>>,
    ) -> Option<Box<PieceNode>> {
        match (left, right) {
            (None, right) => right,
            (left, None) => left,
            (Some(mut left), Some(mut right)) => {
                if left.priority >= right.priority {
                    left.right = Self::merge(left.right.take(), Some(right));
                    left.refresh();
                    Some(left)
                } else {
                    right.left = Self::merge(Some(left), right.left.take());
                    right.refresh();
                    Some(right)
                }
            }
        }
    }

    fn piece_slice(&self, piece: Piece) -> &[u8] {
        match piece.source {
            PieceSource::Original => &self.original[piece.start..piece.start + piece.len],
            PieceSource::Add => &self.add[piece.start..piece.start + piece.len],
        }
    }

    fn piece_byte_to_char(&self, piece: Piece, byte_pos: usize) -> usize {
        let slice = self.piece_slice(piece);
        emacs_byte_to_char_in_slice(slice, byte_pos, self.multibyte, "piece tree byte boundary")
    }

    fn byte_to_char_in_node(&self, tree: &Option<Box<PieceNode>>, byte_pos: usize) -> usize {
        let Some(node) = tree.as_ref() else {
            return 0;
        };

        let left = node_metrics(&node.left);
        if byte_pos <= left.z_byte {
            return self.byte_to_char_in_node(&node.left, byte_pos);
        }

        let after_left = byte_pos - left.z_byte;
        if after_left <= node.piece.len {
            return left.z + self.piece_byte_to_char(node.piece, after_left);
        }

        left.z
            + node.piece.chars
            + self.byte_to_char_in_node(&node.right, after_left - node.piece.len)
    }

    fn char_to_byte_in_node(&self, tree: &Option<Box<PieceNode>>, char_pos: usize) -> usize {
        let Some(node) = tree.as_ref() else {
            return 0;
        };

        let left = node_metrics(&node.left);
        if char_pos <= left.z {
            return self.char_to_byte_in_node(&node.left, char_pos);
        }

        let after_left = char_pos - left.z;
        if after_left <= node.piece.chars {
            return left.z_byte
                + emacs_char_to_byte_in_slice(
                    self.piece_slice(node.piece),
                    after_left,
                    self.multibyte,
                );
        }

        left.z_byte
            + node.piece.len
            + self.char_to_byte_in_node(&node.right, after_left - node.piece.chars)
    }

    fn for_each_range<E>(
        &self,
        tree: &Option<Box<PieceNode>>,
        start: usize,
        end: usize,
        f: &mut impl FnMut(&[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        if start >= end {
            return Ok(());
        }
        let Some(node) = tree.as_ref() else {
            return Ok(());
        };

        let left = node_metrics(&node.left);
        if start < left.z_byte {
            self.for_each_range(&node.left, start, end.min(left.z_byte), f)?;
        }

        let piece_start = left.z_byte;
        let piece_end = piece_start + node.piece.len;
        if start < piece_end && end > piece_start {
            let local_start = start.max(piece_start) - piece_start;
            let local_end = end.min(piece_end) - piece_start;
            f(&self.piece_slice(node.piece)[local_start..local_end])?;
        }

        if end > piece_end {
            self.for_each_range(
                &node.right,
                start.saturating_sub(piece_end),
                end - piece_end,
                f,
            )?;
        }

        Ok(())
    }

    fn contiguous_slice(&self, start: usize, end: usize) -> Option<&[u8]> {
        if start == end {
            return Some(&[]);
        }
        self.contiguous_slice_in_node(&self.root, start, end)
    }

    fn contiguous_slice_in_node(
        &self,
        tree: &Option<Box<PieceNode>>,
        start: usize,
        end: usize,
    ) -> Option<&[u8]> {
        let node = tree.as_ref()?;
        let left = node_metrics(&node.left);
        if end <= left.z_byte {
            return self.contiguous_slice_in_node(&node.left, start, end);
        }

        let piece_start = left.z_byte;
        let piece_end = piece_start + node.piece.len;
        if start >= piece_end {
            return self.contiguous_slice_in_node(&node.right, start - piece_end, end - piece_end);
        }

        if start >= piece_start && end <= piece_end {
            let local_start = start - piece_start;
            let local_end = end - piece_start;
            return Some(&self.piece_slice(node.piece)[local_start..local_end]);
        }

        None
    }
}

impl fmt::Display for PieceTreeTextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text_range(0, self.len()))
    }
}

impl fmt::Debug for PieceTreeTextBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PieceTreeTextBackend")
            .field("bytes", &self.len())
            .field("chars", &self.metrics().z)
            .field("multibyte", &self.multibyte)
            .finish()
    }
}

fn node_metrics(node: &Option<Box<PieceNode>>) -> BufferTextMetrics {
    node.as_ref().map(|node| node.metrics).unwrap_or_default()
}

fn piece_metrics(piece: Piece) -> BufferTextMetrics {
    BufferTextMetrics {
        z: piece.chars,
        z_byte: piece.len,
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

#[inline]
fn emacs_char_count_bytes(bytes: &[u8], multibyte: bool) -> usize {
    if multibyte {
        crate::emacs_core::emacs_char::chars_in_multibyte(bytes)
    } else {
        bytes.len()
    }
}

#[inline]
fn emacs_char_to_byte_in_slice(bytes: &[u8], char_pos: usize, multibyte: bool) -> usize {
    if multibyte {
        crate::emacs_core::emacs_char::char_to_byte_pos(bytes, char_pos)
    } else {
        char_pos.min(bytes.len())
    }
}

#[inline]
fn emacs_byte_to_char_in_slice(
    bytes: &[u8],
    byte_pos: usize,
    multibyte: bool,
    context: &str,
) -> usize {
    if !multibyte {
        return byte_pos.min(bytes.len());
    }
    assert!(
        is_emacs_char_boundary(bytes, byte_pos, multibyte),
        "{context}: byte_pos ({byte_pos}) is not an Emacs character boundary",
    );
    crate::emacs_core::emacs_char::byte_to_char_pos(bytes, byte_pos)
}

#[inline]
fn is_emacs_char_boundary(bytes: &[u8], byte_pos: usize, multibyte: bool) -> bool {
    if byte_pos > bytes.len() {
        return false;
    }
    if !multibyte || byte_pos == 0 || byte_pos == bytes.len() {
        return true;
    }
    (bytes[byte_pos] & 0xC0) != 0x80
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::gap_buffer::GapBuffer;

    fn assert_matches_gap(piece: &PieceTreeTextBackend, gap: &GapBuffer) {
        assert_eq!(piece.len(), gap.len());
        assert_eq!(piece.metrics().z, gap.char_count());
        assert_eq!(piece.to_string(), gap.to_string());

        let mut piece_bytes = Vec::new();
        let mut gap_bytes = Vec::new();
        piece.copy_emacs_bytes_to(0, piece.len(), &mut piece_bytes);
        gap.copy_emacs_bytes_to(0, gap.len(), &mut gap_bytes);
        assert_eq!(piece_bytes, gap_bytes);
    }

    #[test]
    fn piece_tree_reports_metrics_and_layout() {
        let backend = PieceTreeTextBackend::from_str("éz");
        assert_eq!(
            backend.kind(),
            crate::buffer::BufferTextBackendKind::PieceTree
        );
        assert_eq!(
            backend.layout(),
            BufferTextBackendLayout::PieceTree(BufferTextMetrics { z: 2, z_byte: 3 })
        );
        assert_eq!(backend.char_to_byte(1), "é".len());
        assert_eq!(backend.byte_to_char("é".len()), 1);
    }

    #[test]
    fn piece_tree_insert_delete_and_replace_match_gap_buffer() {
        let mut piece = PieceTreeTextBackend::from_str("abécd日本");
        let mut gap = GapBuffer::from_str("abécd日本");
        assert_matches_gap(&piece, &gap);

        let pos = piece.char_to_byte(2);
        piece.insert_str(pos, "XYZ");
        gap.insert_str(pos, "XYZ");
        assert_matches_gap(&piece, &gap);

        let start = piece.char_to_byte(1);
        let end = piece.char_to_byte(5);
        let nchars = piece.byte_to_char(end) - piece.byte_to_char(start);
        piece.delete_range_both(start, end, nchars);
        gap.delete_range_both(start, end, nchars);
        assert_matches_gap(&piece, &gap);

        let start = piece.char_to_byte(1);
        let end = piece.char_to_byte(2);
        piece.replace_same_len_emacs_bytes(start, end, "ß".as_bytes());
        gap.replace_same_len_emacs_bytes(start, end, "ß".as_bytes());
        assert_matches_gap(&piece, &gap);
    }

    #[test]
    fn piece_tree_visits_piece_chunks_without_coalescing() {
        let mut backend = PieceTreeTextBackend::from_str("abcdef");
        backend.insert_str(3, "XY");
        backend.delete_range(4, 5);

        let mut chunks = Vec::new();
        backend
            .for_each_emacs_byte_chunk(1, 7, |chunk| {
                chunks.push(chunk.to_vec());
                Ok::<(), ()>(())
            })
            .unwrap();
        assert_eq!(chunks, vec![b"bc".to_vec(), b"X".to_vec(), b"def".to_vec()]);
    }

    #[test]
    fn piece_tree_unibyte_raw_bytes_round_trip() {
        let raw = vec![0xFF, b'A', 0x80];
        let mut backend = PieceTreeTextBackend::from_emacs_bytes(&raw, false);
        backend.insert_emacs_bytes(1, &[b'\n']);

        assert!(!backend.is_multibyte());
        assert_eq!(backend.metrics().z, 4);
        assert_eq!(backend.metrics().z_byte, 4);
        assert_eq!(backend.byte_to_char(3), 3);
        assert_eq!(backend.char_to_byte(4), 4);

        let mut bytes = Vec::new();
        backend.copy_emacs_bytes_to(0, backend.len(), &mut bytes);
        assert_eq!(bytes, vec![0xFF, b'\n', b'A', 0x80]);
    }
}
