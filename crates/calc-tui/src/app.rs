use std::io;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use ratatui::prelude::*;
use ratatui::backend::CrosstermBackend;

use crate::clipboard;
use crate::input::{self, is_word_char, char_to_byte_pos};
use crate::mode::{Config, EditStyle, Mode};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAction {
    SaveAs,
    ConfirmQuit,
    ConfirmCloseTab,
}

pub struct Prompt {
    pub label: &'static str,
    pub buffer: String,
    pub action: PromptAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Line,
    Char,
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub anchor_y: usize,
    pub anchor_x: usize,
    pub kind: SelectionKind,
}

// ── Undo / Redo ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct UndoSnapshot {
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
}

// ── Per-buffer state ─────────────────────────────────────────────────────────

pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    /// Remembered column for vertical movement (like vim's "virtual column").
    /// Set to `None` when cursor_x changes from non-vertical actions.
    pub desired_x: Option<usize>,
    pub scroll_offset: usize,
    pub results: Vec<calc_core::LineResult>,
    pub file_path: Option<String>,
    pub dirty: bool,
    pub selection: Option<Selection>,
    pub yank_buffer: Vec<String>,
    pub undo_stack: Vec<UndoSnapshot>,
    pub redo_stack: Vec<UndoSnapshot>,
}

impl Buffer {
    pub fn new() -> Self {
        let mut buf = Self {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
            desired_x: None,
            scroll_offset: 0,
            results: vec![],
            file_path: None,
            dirty: false,
            selection: None,
            yank_buffer: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        buf.evaluate();
        buf
    }

    pub fn tab_name(&self) -> String {
        let name = match &self.file_path {
            Some(p) => p.rsplit('/').next().unwrap_or(p.as_str()).to_string(),
            None => "untitled".to_string(),
        };
        if self.dirty {
            format!("{}*", name)
        } else {
            name
        }
    }

    pub fn evaluate(&mut self) {
        let input = self.lines.join("\n");
        self.results = calc_core::evaluate(&input);
    }
}

// ── EasyMotion ───────────────────────────────────────────────────────────────

pub struct EasyMotionState {
    pub search: String,
    pub matches: Vec<(usize, usize)>, // (line_idx, char_col)
    pub labels: Vec<char>,            // parallel to matches
}

// ── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub buffers: Vec<Buffer>,
    pub active_tab: usize,
    pub should_quit: bool,
    pub message: Option<String>,
    pub config: Config,
    pub mode: Mode,
    pub command_buffer: String,
    pub pending_key: Option<String>,
    pub prompt: Option<Prompt>,
    pub easy_motion: Option<EasyMotionState>,
    pub last_visible_height: usize,
    /// Cached layout rects for mouse hit-testing (set during render).
    pub layout_editor_area: Option<ratatui::prelude::Rect>,
    pub layout_tab_bar: Option<ratatui::prelude::Rect>,
    pub layout_gutter_width: u16,
    pub layout_results_area: Option<ratatui::prelude::Rect>,
    /// For double-click detection.
    pub last_click: Option<(std::time::Instant, u16, u16)>,
    /// Vim count prefix buffer (e.g. "15" in "15G").
    pub count_buffer: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mode = match config.edit_style {
            EditStyle::Vim => Mode::Normal,
            EditStyle::Simple => Mode::Insert,
        };
        let msg = match config.edit_style {
            EditStyle::Vim => "Calc (vim) — :q quit | :w save | i insert".to_string(),
            EditStyle::Simple => "Calc — Ctrl+Q quit | Ctrl+S save | Ctrl+N new".to_string(),
        };
        Self {
            buffers: vec![Buffer::new()],
            active_tab: 0,
            should_quit: false,
            message: Some(msg),
            config,
            mode,
            command_buffer: String::new(),
            pending_key: None,
            prompt: None,
            easy_motion: None,
            last_visible_height: 20,
            layout_editor_area: None,
            layout_tab_bar: None,
            layout_gutter_width: 0,
            layout_results_area: None,
            last_click: None,
            count_buffer: String::new(),
        }
    }

    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| ui::render(f, self))?;

            // Set cursor shape: bar in Insert/Simple, block otherwise
            let cursor_style = match self.config.edit_style {
                EditStyle::Simple => SetCursorStyle::SteadyBar,
                EditStyle::Vim => match self.mode {
                    Mode::Insert => SetCursorStyle::SteadyBar,
                    _ => SetCursorStyle::SteadyBlock,
                },
            };
            crossterm::execute!(io::stdout(), cursor_style)?;

            if self.should_quit {
                break;
            }

            if event::poll(std::time::Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key),
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    // ── Key dispatch ─────────────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) {
        if self.prompt.is_some() {
            self.handle_prompt_key(key);
            return;
        }

        match self.config.edit_style {
            EditStyle::Simple => self.handle_key_simple(key),
            EditStyle::Vim => self.handle_key_vim(key),
        }
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        if let Some(ref prompt) = self.prompt {
            if prompt.action == PromptAction::ConfirmQuit {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.prompt = None;
                        self.should_quit = true;
                    }
                    _ => {
                        self.prompt = None;
                        self.message = Some("Quit cancelled".to_string());
                    }
                }
                return;
            }
            if prompt.action == PromptAction::ConfirmCloseTab {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.prompt = None;
                        self.close_tab_force();
                    }
                    _ => {
                        self.prompt = None;
                        self.message = Some("Close cancelled".to_string());
                    }
                }
                return;
            }
        }

        match key.code {
            KeyCode::Esc => {
                self.prompt = None;
            }
            KeyCode::Enter => {
                if let Some(prompt) = self.prompt.take() {
                    let value = prompt.buffer.trim().to_string();
                    if value.is_empty() {
                        self.message = Some("Cancelled".to_string());
                        return;
                    }
                    match prompt.action {
                        PromptAction::SaveAs => {
                            self.save_as(&value);
                        }
                        PromptAction::ConfirmQuit | PromptAction::ConfirmCloseTab => {
                            unreachable!()
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Some(ref mut prompt) = self.prompt {
                    if prompt.buffer.pop().is_none() {
                        self.prompt = None;
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(ref mut prompt) = self.prompt {
                    prompt.buffer.push(c);
                }
            }
            _ => {}
        }
    }

    fn handle_key_simple(&mut self, key: KeyEvent) {
        let i = self.active_tab;

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    let any_dirty = self.buffers.iter().any(|b| b.dirty);
                    if any_dirty {
                        self.prompt = Some(Prompt {
                            label: "Unsaved changes. Quit? (y/n): ",
                            buffer: String::new(),
                            action: PromptAction::ConfirmQuit,
                        });
                    } else {
                        self.should_quit = true;
                    }
                    return;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.save();
                    return;
                }
                KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.new_buffer();
                    return;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.close_tab();
                    return;
                }
                KeyCode::Char('z') => {
                    self.undo();
                    return;
                }
                KeyCode::Char('Z') => {
                    self.redo();
                    return;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.copy_selection_or_line();
                    return;
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.save_undo_snapshot();
                    self.cut_selection_or_line();
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    self.save_undo_snapshot();
                    self.paste_from_clipboard();
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.copy_result();
                    return;
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.save_undo_snapshot();
                    if self.buffers[i].selection.is_some() {
                        self.cut_selection_or_line();
                    } else {
                        let removed = self.delete_line();
                        clipboard::copy(&removed);
                        self.buffers[i].yank_buffer = vec![removed];
                    }
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                KeyCode::Delete => {
                    self.save_undo_snapshot();
                    self.delete_word_forward();
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                KeyCode::Backspace => {
                    self.save_undo_snapshot();
                    self.delete_word_backward();
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                KeyCode::PageDown => {
                    self.next_tab();
                    return;
                }
                KeyCode::PageUp => {
                    self.prev_tab();
                    return;
                }
                _ => {}
            }
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            match key.code {
                KeyCode::Delete => {
                    self.save_undo_snapshot();
                    self.delete_word_forward();
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                KeyCode::Backspace => {
                    self.save_undo_snapshot();
                    self.delete_word_backward();
                    self.buffers[i].dirty = true;
                    self.evaluate();
                    return;
                }
                _ => {}
            }
        }

        // Any Shift combo: start or extend character-level selection
        let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if has_shift {
            if self.buffers[i].selection.is_none() {
                let cy = self.buffers[i].cursor_y;
                let cx = self.buffers[i].cursor_x;
                self.buffers[i].selection = Some(Selection {
                    anchor_y: cy,
                    anchor_x: cx,
                    kind: SelectionKind::Char,
                });
            }

            match key.code {
                KeyCode::Left if has_ctrl => {
                    input::move_word_backward(self);
                    return;
                }
                KeyCode::Right if has_ctrl => {
                    input::move_word_forward(self);
                    return;
                }
                KeyCode::Home if has_ctrl => {
                    self.buffers[i].cursor_y = 0;
                    self.buffers[i].cursor_x = 0;
                    return;
                }
                KeyCode::End if has_ctrl => {
                    self.buffers[i].cursor_y = self.buffers[i].lines.len() - 1;
                    let cy = self.buffers[i].cursor_y;
                    self.buffers[i].cursor_x = self.buffers[i].lines[cy].chars().count();
                    return;
                }
                KeyCode::Up => {
                    self.move_up();
                    return;
                }
                KeyCode::Down => {
                    self.move_down();
                    return;
                }
                KeyCode::Left => {
                    if self.buffers[i].cursor_x > 0 {
                        self.buffers[i].cursor_x -= 1;
                    } else if self.buffers[i].cursor_y > 0 {
                        self.buffers[i].cursor_y -= 1;
                        let cy = self.buffers[i].cursor_y;
                        self.buffers[i].cursor_x = self.buffers[i].lines[cy].chars().count();
                    }
                    return;
                }
                KeyCode::Right => {
                    let cy = self.buffers[i].cursor_y;
                    let line_len = self.buffers[i].lines[cy].chars().count();
                    if self.buffers[i].cursor_x < line_len {
                        self.buffers[i].cursor_x += 1;
                    } else if self.buffers[i].cursor_y + 1 < self.buffers[i].lines.len() {
                        self.buffers[i].cursor_y += 1;
                        self.buffers[i].cursor_x = 0;
                    }
                    return;
                }
                KeyCode::Home => {
                    self.buffers[i].cursor_x = 0;
                    return;
                }
                KeyCode::End => {
                    let cy = self.buffers[i].cursor_y;
                    self.buffers[i].cursor_x = self.buffers[i].lines[cy].chars().count();
                    return;
                }
                _ => {}
            }
        }

        // Any non-shift key clears selection
        if self.buffers[i].selection.is_some() && !has_shift {
            self.buffers[i].selection = None;
        }

        let snap = self.take_snapshot();
        let changed = input::handle_insert_key(self, key);
        if changed {
            self.push_snapshot(snap);
            self.buffers[self.active_tab].dirty = true;
            self.evaluate();
        }
    }

    fn handle_key_vim(&mut self, key: KeyEvent) {
        let i = self.active_tab;
        match self.mode {
            Mode::Normal => {
                let snap = self.take_snapshot();
                let changed = input::handle_normal_key(self, key);
                if changed {
                    self.push_snapshot(snap);
                    self.buffers[i].dirty = true;
                    self.evaluate();
                }
            }
            Mode::Insert => {
                // Safety: clear EasyMotion if mode changed away from Normal
                self.easy_motion = None;
                let snap = self.take_snapshot();
                let changed = input::handle_insert_mode_key(self, key);
                if changed {
                    self.push_snapshot(snap);
                    self.buffers[i].dirty = true;
                    self.evaluate();
                }
            }
            Mode::Visual => {
                // Safety: clear EasyMotion if mode changed away from Normal
                self.easy_motion = None;
                let snap = self.take_snapshot();
                let changed = input::handle_visual_key(self, key);
                if changed {
                    self.push_snapshot(snap);
                    self.buffers[i].dirty = true;
                    self.evaluate();
                }
            }
            Mode::Command => {
                // Safety: clear EasyMotion if mode changed away from Normal
                self.easy_motion = None;
                input::handle_command_key(self, key);
            }
        }
    }

    // ── Mouse handling ────────────────────────────────────────────────────

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        let col = mouse.column;
        let row = mouse.row;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check tab bar click
                if let Some(tab_rect) = self.layout_tab_bar {
                    if row >= tab_rect.y && row < tab_rect.y + tab_rect.height {
                        self.handle_tab_bar_click(col);
                        self.last_click = None;
                        return;
                    }
                }

                // Check editor area click
                if let Some(editor_rect) = self.layout_editor_area {
                    if col >= editor_rect.x
                        && col < editor_rect.x + editor_rect.width
                        && row >= editor_rect.y
                        && row < editor_rect.y + editor_rect.height
                    {
                        let i = self.active_tab;
                        let line_idx =
                            self.buffers[i].scroll_offset + (row - editor_rect.y) as usize;
                        let char_col = (col - editor_rect.x) as usize;

                        // Detect double-click (same position within 400ms)
                        let is_double = self.last_click.map_or(false, |(t, lc, lr)| {
                            lc == col && lr == row && t.elapsed().as_millis() < 400
                        });

                        if line_idx < self.buffers[i].lines.len() {
                            if is_double {
                                // Double-click: select word under cursor
                                self.last_click = None;
                                let chars: Vec<char> =
                                    self.buffers[i].lines[line_idx].chars().collect();
                                if !chars.is_empty() {
                                    let cx = char_col.min(chars.len() - 1);
                                    let (start, end) = input::find_inner_word(&chars, cx);
                                    self.buffers[i].cursor_y = line_idx;
                                    self.buffers[i].selection = Some(Selection {
                                        anchor_y: line_idx,
                                        anchor_x: start,
                                        kind: SelectionKind::Char,
                                    });
                                    self.buffers[i].cursor_x =
                                        if end > 0 { end - 1 } else { 0 };
                                    if self.config.edit_style == EditStyle::Vim {
                                        self.mode = Mode::Visual;
                                    }
                                }
                            } else {
                                // Single click: move cursor
                                self.last_click =
                                    Some((std::time::Instant::now(), col, row));
                                self.buffers[i].cursor_y = line_idx;
                                let line_len =
                                    self.buffers[i].lines[line_idx].chars().count();
                                if self.mode == Mode::Normal || self.mode == Mode::Visual {
                                    self.buffers[i].cursor_x = if line_len == 0 {
                                        0
                                    } else {
                                        char_col.min(line_len - 1)
                                    };
                                } else {
                                    self.buffers[i].cursor_x = char_col.min(line_len);
                                }
                                // Clear selection/pending state
                                self.buffers[i].selection = None;
                                if self.mode == Mode::Visual {
                                    self.mode = Mode::Normal;
                                }
                            }
                            self.clear_desired_x();
                            self.pending_key = None;
                            self.easy_motion = None;
                        }
                    }
                }

                // Check results area click — copy result to clipboard
                if let Some(results_rect) = self.layout_results_area {
                    if col >= results_rect.x
                        && col < results_rect.x + results_rect.width
                        && row >= results_rect.y
                        && row < results_rect.y + results_rect.height
                    {
                        let i = self.active_tab;
                        let line_idx =
                            self.buffers[i].scroll_offset + (row - results_rect.y) as usize;
                        if let Some(result) = self.buffers[i].results.get(line_idx) {
                            let text = if let Some(ref err) = result.error {
                                err.clone()
                            } else if !result.display.is_empty() {
                                result.display.clone()
                            } else {
                                String::new()
                            };
                            if !text.is_empty() {
                                clipboard::copy(&text);
                                self.message = Some(format!("Copied: {}", text));
                            }
                        }
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(editor_rect) = self.layout_editor_area {
                    if col >= editor_rect.x && row >= editor_rect.y {
                        let i = self.active_tab;

                        // Start selection if not already active
                        if self.buffers[i].selection.is_none() {
                            let cy = self.buffers[i].cursor_y;
                            let cx = self.buffers[i].cursor_x;
                            self.buffers[i].selection = Some(Selection {
                                anchor_y: cy,
                                anchor_x: cx,
                                kind: SelectionKind::Char,
                            });
                            // Enter visual mode in vim
                            if self.config.edit_style == EditStyle::Vim {
                                self.mode = Mode::Visual;
                            }
                        }

                        let line_idx =
                            self.buffers[i].scroll_offset + (row - editor_rect.y) as usize;
                        let char_col = (col - editor_rect.x) as usize;

                        if line_idx < self.buffers[i].lines.len() {
                            self.buffers[i].cursor_y = line_idx;
                            let line_len = self.buffers[i].lines[line_idx].chars().count();
                            self.buffers[i].cursor_x = char_col.min(line_len);
                            self.clear_desired_x();
                        }
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                let i = self.active_tab;
                let scroll_amount = 3usize;
                self.buffers[i].scroll_offset =
                    self.buffers[i].scroll_offset.saturating_sub(scroll_amount);
                if self.buffers[i].cursor_y
                    >= self.buffers[i].scroll_offset + self.last_visible_height
                {
                    self.buffers[i].cursor_y =
                        self.buffers[i].scroll_offset + self.last_visible_height - 1;
                    self.clamp_cursor();
                }
            }
            MouseEventKind::ScrollDown => {
                let i = self.active_tab;
                let scroll_amount = 3usize;
                let max_scroll = self.buffers[i].lines.len().saturating_sub(1);
                self.buffers[i].scroll_offset =
                    (self.buffers[i].scroll_offset + scroll_amount).min(max_scroll);
                if self.buffers[i].cursor_y < self.buffers[i].scroll_offset {
                    self.buffers[i].cursor_y = self.buffers[i].scroll_offset;
                    self.clamp_cursor();
                }
            }
            _ => {}
        }
    }

    fn handle_tab_bar_click(&mut self, col: u16) {
        // Tab labels are rendered as " name " with a space separator.
        // Walk through tabs to find which one was clicked.
        let mut x: u16 = 0;
        for (idx, buf) in self.buffers.iter().enumerate() {
            let label_len = buf.tab_name().len() as u16 + 2; // " name "
            if col >= x && col < x + label_len {
                self.active_tab = idx;
                return;
            }
            x += label_len + 1; // +1 for separator space
        }
    }

    // ── Tab management ───────────────────────────────────────────────────

    pub fn new_buffer(&mut self) {
        self.buffers.push(Buffer::new());
        self.active_tab = self.buffers.len() - 1;
        self.message = Some("New buffer".to_string());
    }

    pub fn next_tab(&mut self) {
        if self.buffers.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.buffers.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.buffers.len() > 1 {
            self.active_tab = if self.active_tab == 0 {
                self.buffers.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn close_tab(&mut self) {
        if self.buffers[self.active_tab].dirty {
            self.prompt = Some(Prompt {
                label: "Unsaved changes. Close tab? (y/n): ",
                buffer: String::new(),
                action: PromptAction::ConfirmCloseTab,
            });
        } else {
            self.close_tab_force();
        }
    }

    pub fn close_tab_force(&mut self) {
        if self.buffers.len() == 1 {
            self.should_quit = true;
            return;
        }
        self.buffers.remove(self.active_tab);
        if self.active_tab >= self.buffers.len() {
            self.active_tab = self.buffers.len() - 1;
        }
        self.message = Some(format!("Tab closed ({} remaining)", self.buffers.len()));
    }

    // ── Evaluate ─────────────────────────────────────────────────────────

    pub fn evaluate(&mut self) {
        self.buffers[self.active_tab].evaluate();
    }

    // ── Undo / Redo ──────────────────────────────────────────────────────

    pub fn save_undo_snapshot(&mut self) {
        let b = &mut self.buffers[self.active_tab];
        b.undo_stack.push(UndoSnapshot {
            lines: b.lines.clone(),
            cursor_x: b.cursor_x,
            cursor_y: b.cursor_y,
        });
        b.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        let b = &mut self.buffers[self.active_tab];
        if let Some(snap) = b.undo_stack.pop() {
            b.redo_stack.push(UndoSnapshot {
                lines: b.lines.clone(),
                cursor_x: b.cursor_x,
                cursor_y: b.cursor_y,
            });
            b.lines = snap.lines;
            b.cursor_x = snap.cursor_x;
            b.cursor_y = snap.cursor_y;
            b.evaluate();
            self.message = Some("Undo".to_string());
        } else {
            self.message = Some("Already at oldest change".to_string());
        }
    }

    pub fn redo(&mut self) {
        let b = &mut self.buffers[self.active_tab];
        if let Some(snap) = b.redo_stack.pop() {
            b.undo_stack.push(UndoSnapshot {
                lines: b.lines.clone(),
                cursor_x: b.cursor_x,
                cursor_y: b.cursor_y,
            });
            b.lines = snap.lines;
            b.cursor_x = snap.cursor_x;
            b.cursor_y = snap.cursor_y;
            b.evaluate();
            self.message = Some("Redo".to_string());
        } else {
            self.message = Some("Already at newest change".to_string());
        }
    }

    fn take_snapshot(&self) -> UndoSnapshot {
        let b = &self.buffers[self.active_tab];
        UndoSnapshot {
            lines: b.lines.clone(),
            cursor_x: b.cursor_x,
            cursor_y: b.cursor_y,
        }
    }

    fn push_snapshot(&mut self, snap: UndoSnapshot) {
        let b = &mut self.buffers[self.active_tab];
        b.undo_stack.push(snap);
        b.redo_stack.clear();
    }

    // ── File I/O ─────────────────────────────────────────────────────────

    pub fn save(&mut self) {
        let i = self.active_tab;
        let path = match &self.buffers[i].file_path {
            Some(p) => p.clone(),
            None => {
                self.prompt = Some(Prompt {
                    label: "Save as: ",
                    buffer: String::new(),
                    action: PromptAction::SaveAs,
                });
                return;
            }
        };
        self.write_to_path(&path);
    }

    pub fn save_as(&mut self, path: &str) {
        let path = if path.contains('.') {
            path.to_string()
        } else {
            format!("{}.calc", path)
        };
        self.buffers[self.active_tab].file_path = Some(path.clone());
        self.write_to_path(&path);
    }

    fn write_to_path(&mut self, path: &str) {
        let i = self.active_tab;
        let content = self.buffers[i].lines.join("\n");
        match std::fs::write(path, &content) {
            Ok(_) => {
                self.buffers[i].dirty = false;
                self.message = Some(format!("Saved to {}", path));
            }
            Err(e) => {
                self.message = Some(format!("Error saving: {}", e));
            }
        }
    }

    pub fn load_file(&mut self, path: &str) -> io::Result<()> {
        let i = self.active_tab;
        self.buffers[i].file_path = Some(path.to_string());

        match std::fs::read_to_string(path) {
            Ok(content) => {
                self.buffers[i].lines =
                    content.lines().map(|l| l.to_string()).collect();
                if self.buffers[i].lines.is_empty() {
                    self.buffers[i].lines.push(String::new());
                }
                self.buffers[i].cursor_x = 0;
                self.buffers[i].cursor_y = 0;
                self.buffers[i].scroll_offset = 0;
                self.buffers[i].dirty = false;
                self.buffers[i].evaluate();
                self.message = Some(format!("Loaded {}", path));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.message = Some(format!("New file: {}", path));
            }
            Err(e) => {
                self.buffers[i].file_path = None;
                return Err(e);
            }
        }
        Ok(())
    }

    // ── Scroll / cursor helpers ──────────────────────────────────────────

    pub fn visible_height(&self, area_height: u16) -> usize {
        let overhead = 3 + if self.buffers.len() > 1 { 1 } else { 0 };
        (area_height as usize).saturating_sub(overhead)
    }

    pub fn ensure_cursor_visible(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        let i = self.active_tab;
        if self.buffers[i].cursor_y < self.buffers[i].scroll_offset {
            self.buffers[i].scroll_offset = self.buffers[i].cursor_y;
        }
        if self.buffers[i].cursor_y >= self.buffers[i].scroll_offset + visible_height {
            self.buffers[i].scroll_offset = self.buffers[i].cursor_y - visible_height + 1;
        }
    }

    /// Move cursor vertically preserving the desired column (like vim).
    pub fn move_up(&mut self) {
        let i = self.active_tab;
        if self.buffers[i].cursor_y == 0 {
            return;
        }
        // Remember current x as desired column if not already set
        if self.buffers[i].desired_x.is_none() {
            self.buffers[i].desired_x = Some(self.buffers[i].cursor_x);
        }
        self.buffers[i].cursor_y -= 1;
        let target = self.buffers[i].desired_x.unwrap();
        let cy = self.buffers[i].cursor_y;
        let line_len = self.buffers[i].lines[cy].chars().count();
        if self.mode == Mode::Normal || self.mode == Mode::Visual {
            self.buffers[i].cursor_x = if line_len == 0 { 0 } else { target.min(line_len - 1) };
        } else {
            self.buffers[i].cursor_x = target.min(line_len);
        }
    }

    pub fn move_down(&mut self) {
        let i = self.active_tab;
        if self.buffers[i].cursor_y + 1 >= self.buffers[i].lines.len() {
            return;
        }
        if self.buffers[i].desired_x.is_none() {
            self.buffers[i].desired_x = Some(self.buffers[i].cursor_x);
        }
        self.buffers[i].cursor_y += 1;
        let target = self.buffers[i].desired_x.unwrap();
        let cy = self.buffers[i].cursor_y;
        let line_len = self.buffers[i].lines[cy].chars().count();
        if self.mode == Mode::Normal || self.mode == Mode::Visual {
            self.buffers[i].cursor_x = if line_len == 0 { 0 } else { target.min(line_len - 1) };
        } else {
            self.buffers[i].cursor_x = target.min(line_len);
        }
    }

    /// Clear desired_x — call this after any horizontal cursor movement or edit.
    pub fn clear_desired_x(&mut self) {
        self.buffers[self.active_tab].desired_x = None;
    }

    pub fn clamp_cursor(&mut self) {
        let mode = self.mode;
        let i = self.active_tab;
        let cy = self.buffers[i].cursor_y;
        let line_len = self.buffers[i].lines[cy].chars().count();
        if mode == Mode::Normal || mode == Mode::Visual {
            if line_len == 0 {
                self.buffers[i].cursor_x = 0;
            } else {
                self.buffers[i].cursor_x = self.buffers[i].cursor_x.min(line_len - 1);
            }
        } else {
            self.buffers[i].cursor_x = self.buffers[i].cursor_x.min(line_len);
        }
    }

    // ── Line operations ──────────────────────────────────────────────────

    pub fn delete_line(&mut self) -> String {
        let i = self.active_tab;
        let cy = self.buffers[i].cursor_y;
        let removed = self.buffers[i].lines.remove(cy);
        if self.buffers[i].lines.is_empty() {
            self.buffers[i].lines.push(String::new());
        }
        if self.buffers[i].cursor_y >= self.buffers[i].lines.len() {
            self.buffers[i].cursor_y = self.buffers[i].lines.len() - 1;
        }
        let new_cy = self.buffers[i].cursor_y;
        let line_len = self.buffers[i].lines[new_cy].chars().count();
        self.buffers[i].cursor_x = self.buffers[i].cursor_x.min(line_len.saturating_sub(1));
        removed
    }

    pub fn yank_line(&mut self) {
        let i = self.active_tab;
        let cy = self.buffers[i].cursor_y;
        let line = self.buffers[i].lines[cy].clone();
        clipboard::copy(&line);
        self.buffers[i].yank_buffer = vec![line];
    }

    pub fn paste_below(&mut self) {
        let i = self.active_tab;
        if self.buffers[i].yank_buffer.is_empty() {
            return;
        }
        let yb = self.buffers[i].yank_buffer.clone();
        let cy = self.buffers[i].cursor_y;
        let idx = (cy + 1).min(self.buffers[i].lines.len());
        for (j, line) in yb.iter().enumerate() {
            self.buffers[i].lines.insert(idx + j, line.clone());
        }
        self.buffers[i].cursor_y = idx;
        self.buffers[i].cursor_x = 0;
    }

    pub fn paste_above(&mut self) {
        let i = self.active_tab;
        if self.buffers[i].yank_buffer.is_empty() {
            return;
        }
        let yb = self.buffers[i].yank_buffer.clone();
        let cy = self.buffers[i].cursor_y;
        for (j, line) in yb.iter().enumerate() {
            self.buffers[i].lines.insert(cy + j, line.clone());
        }
        self.buffers[i].cursor_x = 0;
    }

    // ── Clipboard / selection ────────────────────────────────────────────

    pub fn copy_result(&mut self) {
        let i = self.active_tab;
        let cy = self.buffers[i].cursor_y;
        if let Some(result) = self.buffers[i].results.get(cy) {
            let text = if let Some(ref err) = result.error {
                err.clone()
            } else if !result.display.is_empty() {
                result.display.clone()
            } else {
                String::new()
            };
            if !text.is_empty() {
                clipboard::copy(&text);
                self.message = Some(format!("Copied: {}", text));
            } else {
                self.message = Some("No result to copy".to_string());
            }
        } else {
            self.message = Some("No result to copy".to_string());
        }
    }

    pub fn copy_selection_or_line(&mut self) {
        let i = self.active_tab;
        let text = if let Some(sel) = self.buffers[i].selection.take() {
            self.extract_selection_text(&sel)
        } else {
            let cy = self.buffers[i].cursor_y;
            self.buffers[i].lines[cy].clone()
        };
        self.buffers[i].yank_buffer = vec![text.clone()];
        clipboard::copy(&text);
        self.message = Some("Copied".to_string());
    }

    pub fn cut_selection_or_line(&mut self) {
        let i = self.active_tab;
        if let Some(sel) = self.buffers[i].selection.take() {
            let text = self.extract_selection_text(&sel);
            self.buffers[i].yank_buffer = vec![text.clone()];
            clipboard::copy(&text);
            self.delete_selection_range(&sel);
        } else {
            let removed = self.delete_line();
            self.buffers[self.active_tab].yank_buffer = vec![removed.clone()];
            clipboard::copy(&removed);
        }
    }

    pub fn paste_from_clipboard(&mut self) {
        let i = self.active_tab;
        let yb_text = self.buffers[i].yank_buffer.join("\n");
        let text = clipboard::paste().unwrap_or(yb_text);
        if text.is_empty() {
            return;
        }
        let paste_lines: Vec<&str> = text.split('\n').collect();
        if paste_lines.len() == 1 {
            let cx = self.buffers[i].cursor_x;
            let cy = self.buffers[i].cursor_y;
            let line = &mut self.buffers[i].lines[cy];
            let byte_pos = char_to_byte_pos(line, cx);
            line.insert_str(byte_pos, paste_lines[0]);
            self.buffers[i].cursor_x += paste_lines[0].chars().count();
        } else {
            let cy = self.buffers[i].cursor_y;
            let cx = self.buffers[i].cursor_x;
            let current = self.buffers[i].lines[cy].clone();
            let byte_pos = char_to_byte_pos(&current, cx);
            let before = &current[..byte_pos];
            let after = &current[byte_pos..];

            self.buffers[i].lines[cy] = format!("{}{}", before, paste_lines[0]);

            for (j, pline) in paste_lines[1..paste_lines.len() - 1].iter().enumerate() {
                self.buffers[i].lines.insert(cy + 1 + j, pline.to_string());
            }

            let last_idx = cy + paste_lines.len() - 1;
            let last_paste = paste_lines[paste_lines.len() - 1];
            self.buffers[i]
                .lines
                .insert(last_idx, format!("{}{}", last_paste, after));

            self.buffers[i].cursor_y = last_idx;
            self.buffers[i].cursor_x = last_paste.chars().count();
        }
    }

    // ── Word delete ──────────────────────────────────────────────────────

    pub fn delete_word_forward(&mut self) {
        let i = self.active_tab;
        let cy = self.buffers[i].cursor_y;
        let chars: Vec<char> = self.buffers[i].lines[cy].chars().collect();
        let len = chars.len();

        if self.buffers[i].cursor_x >= len {
            if cy + 1 < self.buffers[i].lines.len() {
                let next = self.buffers[i].lines.remove(cy + 1);
                self.buffers[i].lines[cy].push_str(&next);
            }
            return;
        }

        let start = self.buffers[i].cursor_x;
        let mut end = start;

        // Skip leading whitespace first
        while end < len && chars[end].is_whitespace() {
            end += 1;
        }

        // Then skip word chars or punctuation
        if end < len && is_word_char(chars[end]) {
            while end < len && is_word_char(chars[end]) {
                end += 1;
            }
        } else if end < len && !chars[end].is_whitespace() {
            while end < len && !chars[end].is_whitespace() && !is_word_char(chars[end]) {
                end += 1;
            }
        }

        let line = &mut self.buffers[i].lines[cy];
        let bs = char_to_byte_pos(line, start);
        let be = char_to_byte_pos(line, end);
        line.drain(bs..be);
    }

    pub fn delete_word_backward(&mut self) {
        let i = self.active_tab;
        if self.buffers[i].cursor_x == 0 {
            if self.buffers[i].cursor_y > 0 {
                let cy = self.buffers[i].cursor_y;
                let current = self.buffers[i].lines.remove(cy);
                self.buffers[i].cursor_y -= 1;
                let new_cy = self.buffers[i].cursor_y;
                self.buffers[i].cursor_x = self.buffers[i].lines[new_cy].chars().count();
                self.buffers[i].lines[new_cy].push_str(&current);
            }
            return;
        }

        let cy = self.buffers[i].cursor_y;
        let chars: Vec<char> = self.buffers[i].lines[cy].chars().collect();

        let end = self.buffers[i].cursor_x;
        let mut start = end;

        // Skip whitespace before cursor
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }

        // Skip word chars or punctuation backward
        if start > 0 && is_word_char(chars[start - 1]) {
            while start > 0 && is_word_char(chars[start - 1]) {
                start -= 1;
            }
        } else if start > 0 && !chars[start - 1].is_whitespace() {
            while start > 0 && !chars[start - 1].is_whitespace() && !is_word_char(chars[start - 1])
            {
                start -= 1;
            }
        }

        let line = &mut self.buffers[i].lines[cy];
        let bs = char_to_byte_pos(line, start);
        let be = char_to_byte_pos(line, end);
        line.drain(bs..be);
        self.buffers[i].cursor_x = start;
    }

    // ── Selection helpers ────────────────────────────────────────────────

    pub fn ordered_selection(&self, sel: &Selection) -> ((usize, usize), (usize, usize)) {
        let b = &self.buffers[self.active_tab];
        let a = (sel.anchor_y, sel.anchor_x);
        let c = (b.cursor_y, b.cursor_x);
        if a <= c {
            (a, c)
        } else {
            (c, a)
        }
    }

    pub fn extract_selection_text(&self, sel: &Selection) -> String {
        let b = &self.buffers[self.active_tab];
        if sel.kind == SelectionKind::Line {
            let (start, end) = self.ordered_selection(sel);
            return b.lines[start.0..=end.0].join("\n");
        }
        let ((sy, sx), (ey, ex)) = self.ordered_selection(sel);
        if sy == ey {
            let line = &b.lines[sy];
            let chars: Vec<char> = line.chars().collect();
            chars[sx..ex].iter().collect()
        } else {
            let mut result = String::new();
            let first: Vec<char> = b.lines[sy].chars().collect();
            result.extend(&first[sx..]);
            for j in (sy + 1)..ey {
                result.push('\n');
                result.push_str(&b.lines[j]);
            }
            result.push('\n');
            let last: Vec<char> = b.lines[ey].chars().collect();
            result.extend(&last[..ex]);
            result
        }
    }

    pub fn delete_selection_range(&mut self, sel: &Selection) {
        let ((sy, sx), (ey, ex)) = self.ordered_selection(sel);
        let i = self.active_tab;

        if sel.kind == SelectionKind::Line {
            self.buffers[i].lines.drain(sy..=ey);
            if self.buffers[i].lines.is_empty() {
                self.buffers[i].lines.push(String::new());
            }
            self.buffers[i].cursor_y = sy.min(self.buffers[i].lines.len() - 1);
            let new_cy = self.buffers[i].cursor_y;
            let line_len = self.buffers[i].lines[new_cy].chars().count();
            self.buffers[i].cursor_x = self.buffers[i].cursor_x.min(line_len);
            return;
        }

        if sy == ey {
            let line = &mut self.buffers[i].lines[sy];
            let bs = char_to_byte_pos(line, sx);
            let be = char_to_byte_pos(line, ex);
            line.drain(bs..be);
        } else {
            let first_chars: Vec<char> = self.buffers[i].lines[sy].chars().collect();
            let last_chars: Vec<char> = self.buffers[i].lines[ey].chars().collect();
            let merged: String = first_chars[..sx]
                .iter()
                .chain(last_chars[ex..].iter())
                .collect();
            self.buffers[i].lines[sy] = merged;
            if ey > sy + 1 {
                self.buffers[i].lines.drain((sy + 1)..ey);
            }
            if sy + 1 < self.buffers[i].lines.len() {
                self.buffers[i].lines.remove(sy + 1);
            }
        }
        self.buffers[i].cursor_y = sy;
        self.buffers[i].cursor_x = sx;
    }

    // ── EasyMotion ────────────────────────────────────────────────────────

    pub fn recompute_easy_motion(&mut self) {
        const LABEL_POOL: &[char] = &[
            'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';',
            'w', 'e', 'r', 'u', 'i', 'o', 'p',
        ];

        let em = match self.easy_motion.as_mut() {
            Some(em) => em,
            None => return,
        };

        let search = &em.search;
        if search.is_empty() {
            em.matches.clear();
            em.labels.clear();
            return;
        }

        let b = &self.buffers[self.active_tab];
        let start = b.scroll_offset;
        let end = (start + self.last_visible_height).min(b.lines.len());
        let search_lower = search.to_lowercase();

        let mut matches = Vec::new();
        let mut next_chars = std::collections::HashSet::new();

        for line_idx in start..end {
            let line = &b.lines[line_idx];
            let line_lower = line.to_lowercase();
            let search_len = search_lower.len();
            let mut search_start = 0;
            while let Some(pos) = line_lower[search_start..].find(&search_lower) {
                let char_col = line[..search_start + pos].chars().count();
                matches.push((line_idx, char_col));

                // Collect the char right after the match
                let after_byte = search_start + pos + search_lower.len();
                if after_byte < line_lower.len() {
                    if let Some(c) = line_lower[after_byte..].chars().next() {
                        next_chars.insert(c);
                    }
                }

                search_start += pos + search_len.max(1);
                if search_start >= line_lower.len() {
                    break;
                }
            }
        }

        let available: Vec<char> = LABEL_POOL
            .iter()
            .copied()
            .filter(|c| !next_chars.contains(c))
            .collect();

        let label_count = matches.len().min(available.len());
        let labels: Vec<char> = available[..label_count].to_vec();
        // Truncate matches to the number of available labels
        matches.truncate(label_count);

        em.matches = matches;
        em.labels = labels;
    }

    // ── Command execution ────────────────────────────────────────────────

    pub fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.mode = Mode::Normal;

        if cmd == "q" {
            if self.buffers[self.active_tab].dirty {
                self.message =
                    Some("Unsaved changes. Use :q! to force or :w to save.".to_string());
            } else {
                self.close_tab_force();
            }
        } else if cmd == "q!" {
            self.close_tab_force();
        } else if cmd == "qa" {
            let any_dirty = self.buffers.iter().any(|b| b.dirty);
            if any_dirty {
                self.prompt = Some(Prompt {
                    label: "Unsaved changes. Quit all? (y/n): ",
                    buffer: String::new(),
                    action: PromptAction::ConfirmQuit,
                });
            } else {
                self.should_quit = true;
            }
        } else if cmd == "qa!" {
            self.should_quit = true;
        } else if cmd == "w" {
            self.save();
        } else if let Some(path) = cmd.strip_prefix("w ") {
            let path = path.trim();
            if !path.is_empty() {
                self.save_as(path);
            }
        } else if cmd == "wq" || cmd == "x" {
            self.save();
            if self.buffers[self.active_tab].file_path.is_some() {
                self.close_tab_force();
            }
        } else if cmd == "tabnew" {
            self.new_buffer();
        } else if cmd == "tabn" {
            self.next_tab();
        } else if cmd == "tabp" {
            self.prev_tab();
        } else {
            self.message = Some(format!("Unknown command: :{}", cmd));
        }
    }
}
