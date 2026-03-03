use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, SelectionKind};
use crate::mode::{EditStyle, Mode};
use crate::theme::Theme;

pub fn render(frame: &mut Frame, app: &mut App) {
    let theme = Theme::default();
    let area = frame.area();

    // Ensure the cursor row is within the visible window before rendering
    let visible_h = app.visible_height(area.height);
    app.last_visible_height = visible_h;
    app.ensure_cursor_visible(visible_h);

    // Determine if we need a bottom input bar (command mode or prompt)
    let has_bottom_bar = app.mode == Mode::Command || app.prompt.is_some();
    let has_tab_bar = app.buffers.len() > 1;

    let mut constraints: Vec<Constraint> = Vec::new();
    if has_tab_bar {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Min(3));
    constraints.push(Constraint::Length(1));
    if has_bottom_bar {
        constraints.push(Constraint::Length(1));
    }

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;
    if has_tab_bar {
        app.layout_tab_bar = Some(main_layout[idx]);
        render_tab_bar(frame, app, main_layout[idx], &theme);
        idx += 1;
    } else {
        app.layout_tab_bar = None;
    }

    let content_area = main_layout[idx];
    idx += 1;
    let status_area = main_layout[idx];
    idx += 1;

    // Horizontal split: 60% editor, 40% results
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(content_area);

    // Cache editor layout for mouse hit-testing
    {
        let editor_outer = panes[0];
        // Replicate the Block::inner calculation (1px border on each side)
        let inner = Rect {
            x: editor_outer.x + 1,
            y: editor_outer.y + 1,
            width: editor_outer.width.saturating_sub(2),
            height: editor_outer.height.saturating_sub(2),
        };
        let total_lines = app.buffers[app.active_tab].lines.len();
        let digit_count = if total_lines == 0 {
            1
        } else {
            (total_lines as f64).log10().floor() as usize + 1
        };
        let gutter_width = (digit_count.max(2) + 1) as u16;
        app.layout_gutter_width = gutter_width.min(inner.width);
        app.layout_editor_area = Some(Rect {
            x: inner.x + app.layout_gutter_width,
            y: inner.y,
            width: inner.width.saturating_sub(app.layout_gutter_width),
            height: inner.height,
        });
    }

    render_editor(frame, app, panes[0], &theme);
    render_results(frame, app, panes[1], &theme);
    render_status_bar(frame, app, status_area, &theme);

    if has_bottom_bar {
        let bottom_area = main_layout[idx];
        if app.prompt.is_some() {
            render_prompt_bar(frame, app, bottom_area, &theme);
        } else {
            render_command_bar(frame, app, bottom_area, &theme);
        }
    }
}

fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect, _theme: &Theme) {
    let mut spans = Vec::new();
    for (idx, buf) in app.buffers.iter().enumerate() {
        let name = buf.tab_name();
        let label = format!(" {} ", name);
        let style = if idx == app.active_tab {
            Style::default()
                .fg(Color::Rgb(205, 214, 244))
                .bg(Color::Rgb(69, 71, 90))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Rgb(108, 112, 134))
                .bg(Color::Rgb(24, 24, 37))
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::raw(" "));
    }
    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(Color::Rgb(24, 24, 37)));
    frame.render_widget(paragraph, area);
}

fn render_editor(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let buf = &app.buffers[app.active_tab];

    let title = match &buf.file_path {
        Some(p) => {
            let name = p.rsplit('/').next().unwrap_or(p.as_str());
            if buf.dirty {
                format!(" {} [modified] ", name)
            } else {
                format!(" {} ", name)
            }
        }
        None => {
            if buf.dirty {
                " untitled [modified] ".to_string()
            } else {
                " untitled ".to_string()
            }
        }
    };

    let block = Block::default()
        .title(title)
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_style(theme.border_focused)
        .style(Style::default().bg(theme.editor_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Compute gutter width for line numbers
    let total_lines = buf.lines.len();
    let digit_count = if total_lines == 0 {
        1
    } else {
        (total_lines as f64).log10().floor() as usize + 1
    };
    let gutter_width = digit_count.max(2) + 1; // at least 2 digits + 1 space padding

    let gutter_area = Rect {
        x: inner.x,
        y: inner.y,
        width: (gutter_width as u16).min(inner.width),
        height: inner.height,
    };
    let editor_area = Rect {
        x: inner.x + gutter_area.width,
        y: inner.y,
        width: inner.width.saturating_sub(gutter_area.width),
        height: inner.height,
    };

    let visible_lines = inner.height as usize;
    let start = buf.scroll_offset;
    let end = (start + visible_lines).min(buf.lines.len());

    let sel_style = Style::default()
        .bg(Color::Rgb(88, 91, 112))
        .fg(Color::Rgb(205, 214, 244));

    // Compute ordered selection bounds
    let sel_bounds = buf.selection.as_ref().map(|sel| {
        let a = (sel.anchor_y, sel.anchor_x);
        let b = (buf.cursor_y, buf.cursor_x);
        let (s, e) = if a <= b { (a, b) } else { (b, a) };
        (s, e, sel.kind)
    });

    let gutter_dim = Style::default().fg(Color::Rgb(108, 112, 134));
    let gutter_current = Style::default().fg(Color::Rgb(205, 214, 244));

    let mut gutter_lines: Vec<Line> = Vec::with_capacity(visible_lines);
    let mut text_lines: Vec<Line> = Vec::with_capacity(visible_lines);

    for i in start..end {
        // Gutter: line number
        let num_str = format!("{:>width$} ", i + 1, width = digit_count.max(2));
        let num_style = if i == buf.cursor_y {
            gutter_current
        } else {
            gutter_dim
        };
        gutter_lines.push(Line::from(Span::styled(num_str, num_style)));

        let line_content = &buf.lines[i];

        if let Some(((sy, sx), (ey, ex), kind)) = sel_bounds {
            if kind == SelectionKind::Line {
                if i >= sy && i <= ey {
                    text_lines.push(Line::from(Span::styled(line_content.clone(), sel_style)));
                } else {
                    text_lines
                        .push(Line::from(Span::styled(line_content.clone(), theme.text)));
                }
            } else {
                let chars: Vec<char> = line_content.chars().collect();
                let len = chars.len();

                if i < sy || i > ey {
                    text_lines
                        .push(Line::from(Span::styled(line_content.clone(), theme.text)));
                } else if sy == ey && i == sy {
                    let sel_start = sx.min(len);
                    let sel_end = ex.min(len);
                    let mut spans = Vec::new();
                    if sel_start > 0 {
                        let before: String = chars[..sel_start].iter().collect();
                        spans.push(Span::styled(before, theme.text));
                    }
                    if sel_end > sel_start {
                        let mid: String = chars[sel_start..sel_end].iter().collect();
                        spans.push(Span::styled(mid, sel_style));
                    }
                    if sel_end < len {
                        let after: String = chars[sel_end..].iter().collect();
                        spans.push(Span::styled(after, theme.text));
                    }
                    if spans.is_empty() {
                        spans.push(Span::styled(line_content.clone(), theme.text));
                    }
                    text_lines.push(Line::from(spans));
                } else if i == sy {
                    let sel_start = sx.min(len);
                    let mut spans = Vec::new();
                    if sel_start > 0 {
                        let before: String = chars[..sel_start].iter().collect();
                        spans.push(Span::styled(before, theme.text));
                    }
                    let rest: String = chars[sel_start..].iter().collect();
                    spans.push(Span::styled(rest, sel_style));
                    text_lines.push(Line::from(spans));
                } else if i == ey {
                    let sel_end = ex.min(len);
                    let mut spans = Vec::new();
                    let selected: String = chars[..sel_end].iter().collect();
                    spans.push(Span::styled(selected, sel_style));
                    if sel_end < len {
                        let after: String = chars[sel_end..].iter().collect();
                        spans.push(Span::styled(after, theme.text));
                    }
                    text_lines.push(Line::from(spans));
                } else {
                    text_lines
                        .push(Line::from(Span::styled(line_content.clone(), sel_style)));
                }
            }
        } else {
            text_lines.push(Line::from(Span::styled(line_content.clone(), theme.text)));
        }
    }

    // Fill rows below the last line with blank gutter + tilde
    for _ in end..(start + visible_lines) {
        let blank_gutter = " ".repeat(gutter_width);
        gutter_lines.push(Line::from(Span::styled(blank_gutter, gutter_dim)));
        text_lines.push(Line::from(Span::styled("~", theme.comment)));
    }

    let gutter_paragraph = Paragraph::new(gutter_lines);
    frame.render_widget(gutter_paragraph, gutter_area);

    let paragraph = Paragraph::new(text_lines);
    frame.render_widget(paragraph, editor_area);

    // EasyMotion overlay
    if let Some(ref em) = app.easy_motion {
        let frame_buf = frame.buffer_mut();

        // Dim all editor text
        for y in editor_area.y..editor_area.y + editor_area.height {
            for x in editor_area.x..editor_area.x + editor_area.width {
                if let Some(cell) = frame_buf.cell_mut(Position::new(x, y)) {
                    cell.set_style(Style::default().fg(Color::Rgb(69, 71, 90)));
                }
            }
        }

        // Also dim gutter
        for y in gutter_area.y..gutter_area.y + gutter_area.height {
            for x in gutter_area.x..gutter_area.x + gutter_area.width {
                if let Some(cell) = frame_buf.cell_mut(Position::new(x, y)) {
                    cell.set_style(Style::default().fg(Color::Rgb(69, 71, 90)));
                }
            }
        }

        let search_len = em.search.chars().count();

        // Highlight matched text spans (orange) and overlay labels
        for (idx, &(line_idx, char_col)) in em.matches.iter().enumerate() {
            if line_idx < start || line_idx >= end {
                continue;
            }
            let screen_y = editor_area.y + (line_idx - start) as u16;

            // Highlight the matched text in orange
            for offset in 0..search_len {
                let screen_x = editor_area.x + (char_col + offset) as u16;
                if screen_x < editor_area.x + editor_area.width
                    && screen_y < editor_area.y + editor_area.height
                {
                    if let Some(cell) = frame_buf.cell_mut(Position::new(screen_x, screen_y)) {
                        cell.set_style(
                            Style::default()
                                .fg(Color::Rgb(250, 179, 135))
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }

            // Overlay label at match position (magenta bg, bold white)
            if idx < em.labels.len() {
                let label_x = editor_area.x + char_col as u16;
                if label_x < editor_area.x + editor_area.width
                    && screen_y < editor_area.y + editor_area.height
                {
                    if let Some(cell) = frame_buf.cell_mut(Position::new(label_x, screen_y)) {
                        cell.set_char(em.labels[idx]);
                        cell.set_style(
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Rgb(203, 166, 247))
                                .add_modifier(Modifier::BOLD),
                        );
                    }
                }
            }
        }
    }

    // Position the terminal cursor — but not when in command mode, prompt, or EasyMotion
    if app.mode != Mode::Command && app.prompt.is_none() && app.easy_motion.is_none() {
        let cursor_screen_y =
            editor_area.y + (buf.cursor_y.saturating_sub(buf.scroll_offset)) as u16;
        let cursor_screen_x = editor_area.x + buf.cursor_x as u16;

        if cursor_screen_y < editor_area.y + editor_area.height
            && cursor_screen_x < editor_area.x + editor_area.width
        {
            frame.set_cursor_position(Position::new(cursor_screen_x, cursor_screen_y));
        }
    }
}

fn render_results(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let buf = &app.buffers[app.active_tab];

    let block = Block::default()
        .title(" results ")
        .title_style(theme.title)
        .borders(Borders::ALL)
        .border_style(theme.border)
        .style(Style::default().bg(theme.results_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_lines = inner.height as usize;
    let start = buf.scroll_offset;
    let end = (start + visible_lines).min(buf.lines.len());
    let col_width = inner.width as usize;

    let mut text_lines: Vec<Line> = Vec::with_capacity(visible_lines);

    for i in start..end {
        if let Some(result) = buf.results.get(i) {
            if let Some(ref err) = result.error {
                let msg = format!("ERR: {}", err);
                let truncated = if msg.len() > col_width {
                    msg[..col_width].to_string()
                } else {
                    msg
                };
                text_lines.push(Line::from(Span::styled(truncated, theme.result_error)));
            } else if !result.display.is_empty() {
                let display = &result.display;
                let padded = if display.len() >= col_width {
                    display.clone()
                } else {
                    format!("{:>width$}", display, width = col_width)
                };
                text_lines.push(Line::from(Span::styled(padded, theme.result_value)));
            } else {
                text_lines.push(Line::from(""));
            }
        } else {
            text_lines.push(Line::from(""));
        }
    }

    for _ in end..(start + visible_lines) {
        text_lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(text_lines);
    frame.render_widget(paragraph, inner);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let buf = &app.buffers[app.active_tab];
    let width = area.width as usize;

    let left = if app.config.edit_style == EditStyle::Vim {
        let mode_str = app.mode.display();
        if let Some(ref msg) = app.message {
            if mode_str.is_empty() {
                msg.clone()
            } else {
                format!("{} {}", mode_str, msg)
            }
        } else {
            mode_str.to_string()
        }
    } else {
        app.message.clone().unwrap_or_default()
    };

    let position = format!("Ln {}, Col {} ", buf.cursor_y + 1, buf.cursor_x + 1);

    let left_len = left.len();
    let pos_len = position.len();
    let padding = width.saturating_sub(left_len + pos_len);

    let status = format!("{}{:padding$}{}", left, "", position, padding = padding);

    let paragraph = Paragraph::new(status).style(theme.status_bar);
    frame.render_widget(paragraph, area);
}

fn render_prompt_bar(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    if let Some(ref prompt) = app.prompt {
        let display = format!("{}{}", prompt.label, prompt.buffer);
        let paragraph = Paragraph::new(display).style(theme.text);
        frame.render_widget(paragraph, area);

        let cursor_x = area.x + prompt.label.len() as u16 + prompt.buffer.len() as u16;
        let cursor_y = area.y;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}

fn render_command_bar(frame: &mut Frame, app: &App, area: Rect, theme: &Theme) {
    let cmd_display = format!(":{}", app.command_buffer);
    let paragraph = Paragraph::new(cmd_display).style(theme.text);
    frame.render_widget(paragraph, area);

    let cursor_x = area.x + 1 + app.command_buffer.len() as u16;
    let cursor_y = area.y;
    if cursor_x < area.x + area.width {
        frame.set_cursor_position(Position::new(cursor_x, cursor_y));
    }
}
