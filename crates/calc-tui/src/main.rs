use std::io;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use ratatui::backend::CrosstermBackend;

mod app;
mod clipboard;
mod input;
mod mode;
mod theme;
mod ui;

use mode::{Config, EditStyle};

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Parse flags and positional args
    let mut edit_style = EditStyle::Simple;
    let mut file_path: Option<String> = None;
    let mut eval_mode = false;
    let mut eval_exprs: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--vim" => edit_style = EditStyle::Vim,
            "--simple" => edit_style = EditStyle::Simple,
            "eval" => {
                eval_mode = true;
                eval_exprs = args[i + 1..].to_vec();
                break;
            }
            arg if arg.starts_with('-') => {
                eprintln!("Unknown flag: {}", arg);
                std::process::exit(1);
            }
            arg => {
                file_path = Some(arg.to_string());
            }
        }
        i += 1;
    }

    if eval_mode {
        if eval_exprs.is_empty() {
            eprintln!("Usage: calc eval <expression>");
            std::process::exit(1);
        }
        let input = eval_exprs.join("\n");
        let results = calc_core::evaluate(&input);
        for r in &results {
            if !r.display.is_empty() {
                println!("{}", r.display);
            }
        }
        return Ok(());
    }

    let config = Config { edit_style };
    run_tui(config, file_path)
}

fn run_tui(config: Config, file_path: Option<String>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new(config);
    if let Some(path) = file_path {
        if let Err(e) = app.load_file(&path) {
            app.message = Some(format!("Error loading {}: {}", path, e));
        }
    }
    let res = app.run(&mut terminal);

    restore_terminal(&mut terminal)?;
    res
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
