use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{App, EasyMotionState, Selection, SelectionKind};
use crate::clipboard;
use crate::mode::Mode;

// ---------------------------------------------------------------------------
// Shared insert-style editing (used by Simple mode and Vim Insert mode)
// ---------------------------------------------------------------------------

pub fn handle_insert_key(app: &mut App, key: KeyEvent) -> bool {
    let i = app.active_tab;
    // Clear desired column on any non-vertical action
    if !matches!(key.code, KeyCode::Up | KeyCode::Down) {
        app.clear_desired_x();
    }
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return false;
            }
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line = &mut app.buffers[i].lines[cy];
            let byte_pos = char_to_byte_pos(line, cx);
            line.insert(byte_pos, c);
            app.buffers[i].cursor_x += 1;
            true
        }
        KeyCode::Enter => {
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line = app.buffers[i].lines[cy].clone();
            let byte_pos = char_to_byte_pos(&line, cx);
            let before = line[..byte_pos].to_string();
            let after = line[byte_pos..].to_string();
            app.buffers[i].lines[cy] = before;
            app.buffers[i].lines.insert(cy + 1, after);
            app.buffers[i].cursor_y += 1;
            app.buffers[i].cursor_x = 0;
            true
        }
        KeyCode::Backspace => {
            let cx = app.buffers[i].cursor_x;
            let cy = app.buffers[i].cursor_y;
            if cx > 0 {
                let line = &mut app.buffers[i].lines[cy];
                let byte_start = char_to_byte_pos(line, cx - 1);
                let byte_end = char_to_byte_pos(line, cx);
                line.drain(byte_start..byte_end);
                app.buffers[i].cursor_x -= 1;
                true
            } else if cy > 0 {
                let current = app.buffers[i].lines.remove(cy);
                app.buffers[i].cursor_y -= 1;
                let new_cy = app.buffers[i].cursor_y;
                app.buffers[i].cursor_x = app.buffers[i].lines[new_cy].chars().count();
                app.buffers[i].lines[new_cy].push_str(&current);
                true
            } else {
                false
            }
        }
        KeyCode::Delete => {
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line_len = app.buffers[i].lines[cy].chars().count();
            if cx < line_len {
                let line = &mut app.buffers[i].lines[cy];
                let byte_start = char_to_byte_pos(line, cx);
                let byte_end = char_to_byte_pos(line, cx + 1);
                line.drain(byte_start..byte_end);
                true
            } else if cy + 1 < app.buffers[i].lines.len() {
                let next = app.buffers[i].lines.remove(cy + 1);
                app.buffers[i].lines[cy].push_str(&next);
                true
            } else {
                false
            }
        }
        KeyCode::Left => {
            if app.buffers[i].cursor_x > 0 {
                app.buffers[i].cursor_x -= 1;
            } else if app.buffers[i].cursor_y > 0 {
                app.buffers[i].cursor_y -= 1;
                let cy = app.buffers[i].cursor_y;
                app.buffers[i].cursor_x = app.buffers[i].lines[cy].chars().count();
            }
            app.clear_desired_x();
            false
        }
        KeyCode::Right => {
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            if app.buffers[i].cursor_x < line_len {
                app.buffers[i].cursor_x += 1;
            } else if cy + 1 < app.buffers[i].lines.len() {
                app.buffers[i].cursor_y += 1;
                app.buffers[i].cursor_x = 0;
            }
            app.clear_desired_x();
            false
        }
        KeyCode::Up => {
            app.move_up();
            false
        }
        KeyCode::Down => {
            app.move_down();
            false
        }
        KeyCode::Home => {
            app.buffers[i].cursor_x = 0;
            false
        }
        KeyCode::End => {
            let cy = app.buffers[i].cursor_y;
            app.buffers[i].cursor_x = app.buffers[i].lines[cy].chars().count();
            false
        }
        KeyCode::PageUp => {
            let page = 10usize;
            app.buffers[i].cursor_y = app.buffers[i].cursor_y.saturating_sub(page);
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            app.buffers[i].cursor_x = app.buffers[i].cursor_x.min(line_len);
            false
        }
        KeyCode::PageDown => {
            let page = 10usize;
            let max_y = app.buffers[i].lines.len().saturating_sub(1);
            app.buffers[i].cursor_y = (app.buffers[i].cursor_y + page).min(max_y);
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            app.buffers[i].cursor_x = app.buffers[i].cursor_x.min(line_len);
            false
        }
        KeyCode::Tab => {
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line = &mut app.buffers[i].lines[cy];
            let byte_pos = char_to_byte_pos(line, cx);
            line.insert_str(byte_pos, "  ");
            app.buffers[i].cursor_x += 2;
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Vim Insert mode: Esc goes to Normal, everything else delegates to insert
// ---------------------------------------------------------------------------

pub fn handle_insert_mode_key(app: &mut App, key: KeyEvent) -> bool {
    let i = app.active_tab;
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            if app.buffers[i].cursor_x > 0 {
                app.buffers[i].cursor_x -= 1;
            }
            app.message = None;
            false
        }
        _ => handle_insert_key(app, key),
    }
}

// ---------------------------------------------------------------------------
// Vim Normal mode
// ---------------------------------------------------------------------------

pub fn handle_normal_key(app: &mut App, key: KeyEvent) -> bool {
    // EasyMotion intercept: if active, route all keys there
    if app.easy_motion.is_some() {
        return handle_easy_motion_key(app, key);
    }

    // Ctrl+R for redo must be handled before the Ctrl filter
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if key.code == KeyCode::Char('r') {
            app.redo();
            return false;
        }
        return false;
    }

    let i = app.active_tab;

    // If there's a pending key sequence, handle the combo
    if let Some(pending) = app.pending_key.take() {
        app.count_buffer.clear();
        return handle_pending_key(app, &pending, key);
    }

    // Accumulate digits into count buffer (1-9 always, 0 only if buffer non-empty)
    if let KeyCode::Char(c @ '1'..='9') = key.code {
        app.count_buffer.push(c);
        app.message = Some(app.count_buffer.clone());
        return false;
    }
    if let KeyCode::Char('0') = key.code {
        if !app.count_buffer.is_empty() {
            app.count_buffer.push('0');
            app.message = Some(app.count_buffer.clone());
            return false;
        }
    }

    // Take the count and clear buffer for the command about to execute
    let count: Option<usize> = if app.count_buffer.is_empty() {
        None
    } else {
        let n = app.count_buffer.parse::<usize>().ok();
        app.count_buffer.clear();
        app.message = None;
        n
    };

    match key.code {
        // -- Mode transitions --
        KeyCode::Char('i') => {
            app.mode = Mode::Insert;
            app.message = None;
            false
        }
        KeyCode::Char('a') => {
            app.mode = Mode::Insert;
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            if line_len > 0 {
                app.buffers[i].cursor_x = (app.buffers[i].cursor_x + 1).min(line_len);
            }
            app.message = None;
            false
        }
        KeyCode::Char('A') => {
            app.mode = Mode::Insert;
            let cy = app.buffers[i].cursor_y;
            app.buffers[i].cursor_x = app.buffers[i].lines[cy].chars().count();
            app.message = None;
            false
        }
        KeyCode::Char('o') => {
            let cy = app.buffers[i].cursor_y;
            app.buffers[i].lines.insert(cy + 1, String::new());
            app.buffers[i].cursor_y += 1;
            app.buffers[i].cursor_x = 0;
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        KeyCode::Char('O') => {
            let cy = app.buffers[i].cursor_y;
            app.buffers[i].lines.insert(cy, String::new());
            app.buffers[i].cursor_x = 0;
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        KeyCode::Char('v') => {
            app.mode = Mode::Visual;
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            app.buffers[i].selection = Some(Selection {
                anchor_y: cy,
                anchor_x: cx,
                kind: SelectionKind::Char,
            });
            app.message = None;
            false
        }
        KeyCode::Char('V') => {
            app.mode = Mode::Visual;
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            app.buffers[i].selection = Some(Selection {
                anchor_y: cy,
                anchor_x: cx,
                kind: SelectionKind::Line,
            });
            app.message = None;
            false
        }
        KeyCode::Char(':') => {
            app.mode = Mode::Command;
            app.command_buffer.clear();
            false
        }

        // -- Navigation --
        KeyCode::Char('h') | KeyCode::Left => {
            if app.buffers[i].cursor_x > 0 {
                app.buffers[i].cursor_x -= 1;
            }
            app.clear_desired_x();
            false
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_down();
            false
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_up();
            false
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            if line_len > 0 && app.buffers[i].cursor_x < line_len - 1 {
                app.buffers[i].cursor_x += 1;
            }
            app.clear_desired_x();
            false
        }
        KeyCode::Char('0') | KeyCode::Home => {
            app.buffers[i].cursor_x = 0;
            app.clear_desired_x();
            false
        }
        KeyCode::Char('$') | KeyCode::End => {
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            app.buffers[i].cursor_x = if line_len > 0 { line_len - 1 } else { 0 };
            app.clear_desired_x();
            false
        }
        KeyCode::Char('w') => {
            move_word_forward(app);
            app.clear_desired_x();
            false
        }
        KeyCode::Char('e') => {
            move_word_end(app);
            app.clear_desired_x();
            false
        }
        KeyCode::Char('b') => {
            move_word_backward(app);
            app.clear_desired_x();
            false
        }
        KeyCode::Char('G') => {
            if let Some(n) = count {
                let target = n.saturating_sub(1);
                let max = app.buffers[i].lines.len().saturating_sub(1);
                app.buffers[i].cursor_y = target.min(max);
            } else {
                app.buffers[i].cursor_y = app.buffers[i].lines.len() - 1;
            }
            app.clamp_cursor();
            app.clear_desired_x();
            false
        }
        KeyCode::Char('f') => {
            app.easy_motion = Some(EasyMotionState {
                search: String::new(),
                matches: Vec::new(),
                labels: Vec::new(),
            });
            app.message = Some("EasyMotion: type to search".to_string());
            false
        }

        // -- Undo / Redo --
        KeyCode::Char('u') => {
            app.undo();
            false
        }

        // -- Editing --
        KeyCode::Char('x') => {
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            let cx = app.buffers[i].cursor_x;
            if line_len > 0 && cx < line_len {
                let line = &mut app.buffers[i].lines[cy];
                let byte_start = char_to_byte_pos(line, cx);
                let byte_end = char_to_byte_pos(line, cx + 1);
                line.drain(byte_start..byte_end);
                app.clamp_cursor();
                true
            } else {
                false
            }
        }
        KeyCode::Char('p') => {
            app.paste_below();
            true
        }
        KeyCode::Char('P') => {
            app.paste_above();
            true
        }

        // -- Change (C = change to end of line) --
        KeyCode::Char('C') => {
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line = &mut app.buffers[i].lines[cy];
            let byte_pos = char_to_byte_pos(line, cx);
            line.truncate(byte_pos);
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        KeyCode::Char('s') => {
            // s: delete char under cursor, enter insert mode
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            let cx = app.buffers[i].cursor_x;
            if line_len > 0 && cx < line_len {
                let line = &mut app.buffers[i].lines[cy];
                let byte_start = char_to_byte_pos(line, cx);
                let byte_end = char_to_byte_pos(line, cx + 1);
                line.drain(byte_start..byte_end);
            }
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        KeyCode::Char('S') => {
            // S: clear entire line, enter insert mode (same as cc)
            let cy = app.buffers[i].cursor_y;
            app.buffers[i].lines[cy] = String::new();
            app.buffers[i].cursor_x = 0;
            app.mode = Mode::Insert;
            app.message = None;
            true
        }

        // -- Multi-key sequences --
        KeyCode::Char('d') | KeyCode::Char('y') | KeyCode::Char('g') | KeyCode::Char('c') => {
            if let KeyCode::Char(c) = key.code {
                app.pending_key = Some(c.to_string());
            }
            false
        }

        _ => false,
    }
}

fn handle_pending_key(app: &mut App, pending: &str, key: KeyEvent) -> bool {
    let ch = match key.code {
        KeyCode::Char(c) => c,
        _ => return false,
    };

    match (pending, ch) {
        ("d", 'd') => {
            let removed = app.delete_line();
            clipboard::copy(&removed);
            app.buffers[app.active_tab].yank_buffer = vec![removed];
            true
        }
        ("d", 'i') => {
            // di_ — wait for text object
            app.pending_key = Some("di".to_string());
            false
        }
        ("di", 'w') => {
            // diw: delete inner word
            delete_inner_word(app);
            true
        }
        ("y", 'y') => {
            app.yank_line();
            app.message = Some("1 line yanked".to_string());
            false
        }
        ("y", 'r') => {
            app.copy_result();
            false
        }
        ("g", 'g') => {
            app.buffers[app.active_tab].cursor_y = 0;
            app.clamp_cursor();
            false
        }
        ("g", 't') => {
            app.next_tab();
            false
        }
        ("g", 'T') => {
            app.prev_tab();
            false
        }
        // -- c (change) combos --
        ("c", 'c') => {
            let i = app.active_tab;
            let cy = app.buffers[i].cursor_y;
            app.buffers[i].lines[cy] = String::new();
            app.buffers[i].cursor_x = 0;
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        ("c", 'i') => {
            // ci_ — wait for text object
            app.pending_key = Some("ci".to_string());
            false
        }
        ("ci", 'w') => {
            // ciw: delete inner word, enter insert mode
            delete_inner_word(app);
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        ("c", 'w') | ("c", 'e') => {
            let i = app.active_tab;
            let cy = app.buffers[i].cursor_y;
            let chars: Vec<char> = app.buffers[i].lines[cy].chars().collect();
            let len = chars.len();
            let cx = app.buffers[i].cursor_x;

            if cx < len {
                let mut end = cx;
                if is_word_char(chars[end]) {
                    while end < len && is_word_char(chars[end]) {
                        end += 1;
                    }
                } else if !chars[end].is_whitespace() {
                    while end < len && !chars[end].is_whitespace() && !is_word_char(chars[end]) {
                        end += 1;
                    }
                } else {
                    while end < len && chars[end].is_whitespace() {
                        end += 1;
                    }
                    if end < len && is_word_char(chars[end]) {
                        while end < len && is_word_char(chars[end]) {
                            end += 1;
                        }
                    } else {
                        while end < len && !chars[end].is_whitespace() && !is_word_char(chars[end]) {
                            end += 1;
                        }
                    }
                }

                let line = &mut app.buffers[i].lines[cy];
                let bs = char_to_byte_pos(line, cx);
                let be = char_to_byte_pos(line, end);
                line.drain(bs..be);
            }

            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        ("c", 'b') => {
            app.delete_word_backward();
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        ("c", '$') => {
            let i = app.active_tab;
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line = &mut app.buffers[i].lines[cy];
            let byte_pos = char_to_byte_pos(line, cx);
            line.truncate(byte_pos);
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        ("c", '0') => {
            let i = app.active_tab;
            let cy = app.buffers[i].cursor_y;
            let cx = app.buffers[i].cursor_x;
            let line = &mut app.buffers[i].lines[cy];
            let byte_pos = char_to_byte_pos(line, cx);
            line.drain(..byte_pos);
            app.buffers[i].cursor_x = 0;
            app.mode = Mode::Insert;
            app.message = None;
            true
        }
        _ => false,
    }
}

/// Find the start and end (char indices) of the word under the cursor.
fn find_inner_word(chars: &[char], cx: usize) -> (usize, usize) {
    let len = chars.len();
    if len == 0 {
        return (0, 0);
    }
    let pos = cx.min(len - 1);

    let (mut start, mut end) = (pos, pos);

    if is_word_char(chars[pos]) {
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }
        while end + 1 < len && is_word_char(chars[end + 1]) {
            end += 1;
        }
    } else if !chars[pos].is_whitespace() {
        while start > 0 && !chars[start - 1].is_whitespace() && !is_word_char(chars[start - 1]) {
            start -= 1;
        }
        while end + 1 < len && !chars[end + 1].is_whitespace() && !is_word_char(chars[end + 1]) {
            end += 1;
        }
    } else {
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
        while end + 1 < len && chars[end + 1].is_whitespace() {
            end += 1;
        }
    }

    (start, end + 1)
}

/// Delete the word under the cursor (inner word, like vim `diw`).
fn delete_inner_word(app: &mut App) {
    let i = app.active_tab;
    let cy = app.buffers[i].cursor_y;
    let chars: Vec<char> = app.buffers[i].lines[cy].chars().collect();
    if chars.is_empty() {
        return;
    }
    let cx = app.buffers[i].cursor_x;
    let (start, end) = find_inner_word(&chars, cx);

    let line = &mut app.buffers[i].lines[cy];
    let bs = char_to_byte_pos(line, start);
    let be = char_to_byte_pos(line, end);
    line.drain(bs..be);
    app.buffers[i].cursor_x = start;
}

// ---------------------------------------------------------------------------
// EasyMotion key handler
// ---------------------------------------------------------------------------

fn handle_easy_motion_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            app.easy_motion = None;
            app.message = None;
            false
        }
        KeyCode::Backspace => {
            if let Some(ref mut em) = app.easy_motion {
                if em.search.pop().is_none() {
                    app.easy_motion = None;
                    app.message = None;
                    return false;
                }
            }
            app.recompute_easy_motion();
            false
        }
        KeyCode::Char(c) => {
            // Check if c is a label → jump
            let jump_target = app.easy_motion.as_ref().and_then(|em| {
                em.labels
                    .iter()
                    .position(|&l| l == c)
                    .map(|idx| em.matches[idx])
            });

            if let Some((line_idx, char_col)) = jump_target {
                let i = app.active_tab;
                app.buffers[i].cursor_y = line_idx;
                app.buffers[i].cursor_x = char_col;
                app.clear_desired_x();
                app.easy_motion = None;
                app.message = None;
                false
            } else {
                // Extend search
                if let Some(ref mut em) = app.easy_motion {
                    em.search.push(c);
                }
                app.recompute_easy_motion();
                false
            }
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Vim Visual mode
// ---------------------------------------------------------------------------

pub fn handle_visual_key(app: &mut App, key: KeyEvent) -> bool {
    let i = app.active_tab;
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.buffers[i].selection = None;
            false
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_down();
            false
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_up();
            false
        }
        KeyCode::Char('h') | KeyCode::Left => {
            if app.buffers[i].cursor_x > 0 {
                app.buffers[i].cursor_x -= 1;
            }
            app.clear_desired_x();
            false
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            if line_len > 0 && app.buffers[i].cursor_x < line_len - 1 {
                app.buffers[i].cursor_x += 1;
            }
            app.clear_desired_x();
            false
        }
        KeyCode::Char('w') => {
            move_word_forward(app);
            false
        }
        KeyCode::Char('e') => {
            move_word_end(app);
            false
        }
        KeyCode::Char('b') => {
            move_word_backward(app);
            false
        }
        KeyCode::Char('0') | KeyCode::Home => {
            app.buffers[i].cursor_x = 0;
            false
        }
        KeyCode::Char('$') | KeyCode::End => {
            let cy = app.buffers[i].cursor_y;
            let line_len = app.buffers[i].lines[cy].chars().count();
            app.buffers[i].cursor_x = if line_len > 0 { line_len - 1 } else { 0 };
            false
        }
        KeyCode::Char('G') => {
            app.buffers[i].cursor_y = app.buffers[i].lines.len() - 1;
            app.clamp_cursor();
            false
        }
        KeyCode::Char('y') => {
            if let Some(sel) = app.buffers[i].selection.take() {
                if sel.kind == SelectionKind::Line {
                    let ((sy, _), (ey, _)) = app.ordered_selection(&sel);
                    let yanked = app.buffers[i].lines[sy..=ey].to_vec();
                    clipboard::copy(&yanked.join("\n"));
                    app.buffers[i].yank_buffer = yanked;
                    let count = ey - sy + 1;
                    app.message = Some(format!(
                        "{} line{} yanked",
                        count,
                        if count == 1 { "" } else { "s" }
                    ));
                    app.buffers[i].cursor_y = sy;
                } else {
                    let text = app.extract_selection_text(&sel);
                    clipboard::copy(&text);
                    app.buffers[i].yank_buffer = vec![text];
                    app.message = Some("Yanked".to_string());
                }
                app.mode = Mode::Normal;
                app.clamp_cursor();
            }
            false
        }
        KeyCode::Char('d') => {
            if let Some(sel) = app.buffers[i].selection.take() {
                if sel.kind == SelectionKind::Line {
                    let ((sy, _), (ey, _)) = app.ordered_selection(&sel);
                    let yanked = app.buffers[i].lines[sy..=ey].to_vec();
                    clipboard::copy(&yanked.join("\n"));
                    app.buffers[i].yank_buffer = yanked;
                } else {
                    let text = app.extract_selection_text(&sel);
                    clipboard::copy(&text);
                    app.buffers[i].yank_buffer = vec![text];
                }
                app.delete_selection_range(&sel);
                app.mode = Mode::Normal;
                app.clamp_cursor();
                return true;
            }
            false
        }
        _ => false,
    }
}


// ---------------------------------------------------------------------------
// Vim Command mode
// ---------------------------------------------------------------------------

pub fn handle_command_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.command_buffer.clear();
        }
        KeyCode::Enter => {
            app.execute_command();
        }
        KeyCode::Backspace => {
            if app.command_buffer.pop().is_none() {
                app.mode = Mode::Normal;
            }
        }
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Public word motion wrappers (for simple mode Ctrl+Shift selection)
// ---------------------------------------------------------------------------

pub fn move_word_forward_pub(app: &mut App) {
    move_word_forward(app);
}

pub fn move_word_backward_pub(app: &mut App) {
    move_word_backward(app);
}

pub fn find_inner_word_pub(chars: &[char], cx: usize) -> (usize, usize) {
    find_inner_word(chars, cx)
}

// ---------------------------------------------------------------------------
// Word motion helpers
// ---------------------------------------------------------------------------

fn move_word_forward(app: &mut App) {
    let i = app.active_tab;
    let cy = app.buffers[i].cursor_y;
    let chars: Vec<char> = app.buffers[i].lines[cy].chars().collect();
    let len = chars.len();

    if app.buffers[i].cursor_x >= len {
        if cy + 1 < app.buffers[i].lines.len() {
            app.buffers[i].cursor_y += 1;
            app.buffers[i].cursor_x = 0;
        }
        return;
    }

    let mut pos = app.buffers[i].cursor_x;

    // Skip current word characters
    if pos < len && is_word_char(chars[pos]) {
        while pos < len && is_word_char(chars[pos]) {
            pos += 1;
        }
    } else if pos < len && !chars[pos].is_whitespace() {
        while pos < len && !chars[pos].is_whitespace() && !is_word_char(chars[pos]) {
            pos += 1;
        }
    }

    // Skip whitespace
    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    if pos >= len {
        if cy + 1 < app.buffers[i].lines.len() {
            app.buffers[i].cursor_y += 1;
            app.buffers[i].cursor_x = 0;
        } else {
            app.buffers[i].cursor_x = len.saturating_sub(1);
        }
    } else {
        app.buffers[i].cursor_x = pos;
    }
}

fn move_word_end(app: &mut App) {
    let i = app.active_tab;
    let cy = app.buffers[i].cursor_y;
    let chars: Vec<char> = app.buffers[i].lines[cy].chars().collect();
    let len = chars.len();

    if len == 0 || (app.buffers[i].cursor_x >= len.saturating_sub(1) && cy + 1 < app.buffers[i].lines.len()) {
        if cy + 1 < app.buffers[i].lines.len() {
            app.buffers[i].cursor_y += 1;
            let new_cy = app.buffers[i].cursor_y;
            let next_chars: Vec<char> = app.buffers[i].lines[new_cy].chars().collect();
            let next_len = next_chars.len();
            if next_len == 0 {
                app.buffers[i].cursor_x = 0;
                return;
            }
            let mut pos = 0;
            while pos < next_len && next_chars[pos].is_whitespace() {
                pos += 1;
            }
            if pos < next_len && is_word_char(next_chars[pos]) {
                while pos + 1 < next_len && is_word_char(next_chars[pos + 1]) {
                    pos += 1;
                }
            } else if pos < next_len {
                while pos + 1 < next_len && !next_chars[pos + 1].is_whitespace() && !is_word_char(next_chars[pos + 1]) {
                    pos += 1;
                }
            }
            app.buffers[i].cursor_x = pos;
        }
        return;
    }

    let mut pos = app.buffers[i].cursor_x;

    if pos + 1 < len {
        pos += 1;
    } else {
        return;
    }

    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    if pos >= len {
        app.buffers[i].cursor_x = len - 1;
        return;
    }

    if is_word_char(chars[pos]) {
        while pos + 1 < len && is_word_char(chars[pos + 1]) {
            pos += 1;
        }
    } else {
        while pos + 1 < len && !chars[pos + 1].is_whitespace() && !is_word_char(chars[pos + 1]) {
            pos += 1;
        }
    }

    app.buffers[i].cursor_x = pos;
}

fn move_word_backward(app: &mut App) {
    let i = app.active_tab;
    let cy = app.buffers[i].cursor_y;
    let chars: Vec<char> = app.buffers[i].lines[cy].chars().collect();

    if app.buffers[i].cursor_x == 0 {
        if cy > 0 {
            app.buffers[i].cursor_y -= 1;
            let new_cy = app.buffers[i].cursor_y;
            let prev_len = app.buffers[i].lines[new_cy].chars().count();
            app.buffers[i].cursor_x = prev_len.saturating_sub(1);
        }
        return;
    }

    let mut pos = app.buffers[i].cursor_x;

    if pos > 0 {
        pos -= 1;
    }

    // Skip whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // Skip word characters backward
    if pos > 0 && is_word_char(chars[pos]) {
        while pos > 0 && is_word_char(chars[pos - 1]) {
            pos -= 1;
        }
    } else if pos > 0 && !chars[pos].is_whitespace() {
        while pos > 0 && !chars[pos - 1].is_whitespace() && !is_word_char(chars[pos - 1]) {
            pos -= 1;
        }
    }

    app.buffers[i].cursor_x = pos;
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn char_to_byte_pos(s: &str, char_pos: usize) -> usize {
    s.char_indices()
        .nth(char_pos)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}
