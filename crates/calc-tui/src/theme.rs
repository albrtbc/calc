#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub editor_bg: Color,
    pub results_bg: Color,
    pub text: Style,
    pub result_value: Style,
    pub result_unit: Style,
    pub result_error: Style,
    pub comment: Style,
    pub number: Style,
    pub operator: Style,
    pub keyword: Style,
    pub variable: Style,
    pub function_name: Style,
    pub border: Style,
    pub border_focused: Style,
    pub title: Style,
    pub status_bar: Style,
    pub cursor: Style,
    pub cursorline_editor: Style,
    pub cursorline_results: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            editor_bg: Color::Rgb(30, 30, 46),
            results_bg: Color::Rgb(24, 24, 37),
            text: Style::default().fg(Color::Rgb(205, 214, 244)),
            result_value: Style::default().fg(Color::Rgb(166, 227, 161)),
            result_unit: Style::default().fg(Color::Rgb(137, 180, 250)),
            result_error: Style::default().fg(Color::Rgb(243, 139, 168)),
            comment: Style::default().fg(Color::Rgb(108, 112, 134)),
            number: Style::default().fg(Color::Rgb(250, 179, 135)),
            operator: Style::default().fg(Color::Rgb(148, 226, 213)),
            keyword: Style::default().fg(Color::Rgb(203, 166, 247)),
            variable: Style::default().fg(Color::Rgb(205, 214, 244)),
            function_name: Style::default().fg(Color::Rgb(137, 180, 250)),
            border: Style::default().fg(Color::Rgb(69, 71, 90)),
            border_focused: Style::default().fg(Color::Rgb(137, 180, 250)),
            title: Style::default()
                .fg(Color::Rgb(205, 214, 244))
                .add_modifier(Modifier::BOLD),
            status_bar: Style::default()
                .fg(Color::Rgb(166, 173, 200))
                .bg(Color::Rgb(24, 24, 37)),
            cursor: Style::default()
                .bg(Color::Rgb(205, 214, 244))
                .fg(Color::Rgb(30, 30, 46)),
            cursorline_editor: Style::default().bg(Color::Rgb(40, 40, 56)),
            cursorline_results: Style::default().bg(Color::Rgb(33, 33, 47)),
        }
    }
}
