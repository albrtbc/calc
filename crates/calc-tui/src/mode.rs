#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Command,
}

impl Mode {
    pub fn display(&self) -> &'static str {
        match self {
            Mode::Normal => "-- NORMAL --",
            Mode::Insert => "-- INSERT --",
            Mode::Visual => "-- VISUAL --",
            Mode::Command => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditStyle {
    Simple,
    Vim,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub edit_style: EditStyle,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            edit_style: EditStyle::Simple,
        }
    }
}
