// The previous `window_cursor_visual_match_uses_slot_identity` test covered the
// phys/visual dedup helper, which no longer exists: cursors are unified into a
// single per-window list (the selected window's entry is `active`), so the
// content backend draws every entry without deduplicating against a separate
// phys_cursor. There is nothing backend-specific left to assert here.
