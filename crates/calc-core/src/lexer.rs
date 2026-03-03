use crate::error::{CalcError, Result};

/// Keywords recognised by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    In,
    To,
    As,
    Of,
}

/// All token kinds produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    Ident(String),
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Eq,
    // Delimiters
    LParen,
    RParen,
    Comma,
    Colon,
    // Special
    Comment(String),
    Keyword(Keyword),
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn current_pos(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    fn read_line_comment(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            s.push(ch);
            self.advance();
        }
        s
    }

    fn read_number(&mut self) -> Result<f64> {
        let start_line = self.line;
        let start_col = self.col;

        // Check for hex / binary / octal prefix
        if self.peek() == Some('0') {
            if let Some(next) = self.peek_at(1) {
                match next {
                    'x' | 'X' => {
                        self.advance(); // '0'
                        self.advance(); // 'x'
                        return self.read_hex(start_line, start_col);
                    }
                    'b' | 'B' => {
                        self.advance(); // '0'
                        self.advance(); // 'b'
                        return self.read_binary(start_line, start_col);
                    }
                    'o' | 'O' => {
                        self.advance(); // '0'
                        self.advance(); // 'o'
                        return self.read_octal(start_line, start_col);
                    }
                    _ => {}
                }
            }
        }

        // Regular decimal (possibly scientific)
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                s.push(ch);
                self.advance();
            } else if ch == ',' && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                // Treat comma as decimal separator when followed by a digit (e.g. 3,5 → 3.5)
                s.push('.');
                self.advance();
            } else if ch == '_' {
                // Allow underscores as separators (ignored)
                self.advance();
            } else {
                break;
            }
        }

        // Optional scientific notation: e or E followed by optional sign and digits
        if let Some(e) = self.peek() {
            if e == 'e' || e == 'E' {
                let next = self.peek_at(1);
                let is_sci = matches!(next, Some('+') | Some('-') | Some('0'..='9'));
                if is_sci {
                    s.push(e);
                    self.advance();
                    if let Some(sign) = self.peek() {
                        if sign == '+' || sign == '-' {
                            s.push(sign);
                            self.advance();
                        }
                    }
                    while let Some(d) = self.peek() {
                        if d.is_ascii_digit() {
                            s.push(d);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        s.parse::<f64>().map_err(|_| {
            CalcError::new(format!("Invalid number literal: {}", s), start_line, start_col)
        })
    }

    fn read_hex(&mut self, line: usize, col: usize) -> Result<f64> {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_hexdigit() || ch == '_' {
                if ch != '_' {
                    s.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err(CalcError::new("Expected hex digits after 0x", line, col));
        }
        u64::from_str_radix(&s, 16)
            .map(|v| v as f64)
            .map_err(|_| CalcError::new(format!("Invalid hex literal: 0x{}", s), line, col))
    }

    fn read_binary(&mut self, line: usize, col: usize) -> Result<f64> {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == '0' || ch == '1' || ch == '_' {
                if ch != '_' {
                    s.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err(CalcError::new("Expected binary digits after 0b", line, col));
        }
        u64::from_str_radix(&s, 2)
            .map(|v| v as f64)
            .map_err(|_| CalcError::new(format!("Invalid binary literal: 0b{}", s), line, col))
    }

    fn read_octal(&mut self, line: usize, col: usize) -> Result<f64> {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ('0'..='7').contains(&ch) || ch == '_' {
                if ch != '_' {
                    s.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err(CalcError::new("Expected octal digits after 0o", line, col));
        }
        u64::from_str_radix(&s, 8)
            .map(|v| v as f64)
            .map_err(|_| CalcError::new(format!("Invalid octal literal: 0o{}", s), line, col))
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn classify_ident(&self, s: &str) -> TokenKind {
        match s {
            "in" => TokenKind::Keyword(Keyword::In),
            "to" => TokenKind::Keyword(Keyword::To),
            "as" => TokenKind::Keyword(Keyword::As),
            "of" => TokenKind::Keyword(Keyword::Of),
            _ => TokenKind::Ident(s.to_string()),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();

        loop {
            self.skip_whitespace();

            let (line, col) = self.current_pos();

            let ch = match self.peek() {
                None => {
                    tokens.push(Token { kind: TokenKind::Eof, line, col });
                    break;
                }
                Some(c) => c,
            };

            // Newline
            if ch == '\n' {
                self.advance();
                tokens.push(Token { kind: TokenKind::Newline, line, col });
                continue;
            }

            // Line comments: # or //
            if ch == '#' {
                self.advance(); // consume '#'
                let comment = self.read_line_comment();
                tokens.push(Token { kind: TokenKind::Comment(comment), line, col });
                continue;
            }
            if ch == '/' && self.peek_at(1) == Some('/') {
                self.advance(); // '/'
                self.advance(); // '/'
                let comment = self.read_line_comment();
                tokens.push(Token { kind: TokenKind::Comment(comment), line, col });
                continue;
            }

            // Numbers
            if ch.is_ascii_digit() || (ch == '.' && self.peek_at(1).map_or(false, |c| c.is_ascii_digit())) {
                let n = self.read_number()?;
                tokens.push(Token { kind: TokenKind::Number(n), line, col });
                continue;
            }

            // Identifiers / keywords
            // Also allow identifiers starting with degree sign for unit names like °C
            if ch.is_alphabetic() || ch == '_' || ch == '°' {
                let mut s = String::new();
                if ch == '°' {
                    s.push(ch);
                    self.advance();
                    // expect C or F after °
                    if let Some(next) = self.peek() {
                        if next == 'C' || next == 'F' {
                            s.push(next);
                            self.advance();
                        }
                    }
                } else {
                    s = self.read_ident();
                }
                let kind = self.classify_ident(&s);
                tokens.push(Token { kind, line, col });
                continue;
            }

            // Single-char tokens
            self.advance();
            let kind = match ch {
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '^' => TokenKind::Caret,
                '=' => TokenKind::Eq,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                other => {
                    return Err(CalcError::new(
                        format!("Unexpected character: {:?}", other),
                        line,
                        col,
                    ));
                }
            };
            tokens.push(Token { kind, line, col });
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(input: &str) -> Vec<TokenKind> {
        let mut l = Lexer::new(input);
        l.tokenize().unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn test_basic_tokens() {
        let k = kinds("1 + 2");
        assert_eq!(k, vec![
            TokenKind::Number(1.0),
            TokenKind::Plus,
            TokenKind::Number(2.0),
            TokenKind::Eof,
        ]);
    }

    #[test]
    fn test_hex() {
        let k = kinds("0xFF");
        assert_eq!(k, vec![TokenKind::Number(255.0), TokenKind::Eof]);
    }

    #[test]
    fn test_binary() {
        let k = kinds("0b1010");
        assert_eq!(k, vec![TokenKind::Number(10.0), TokenKind::Eof]);
    }

    #[test]
    fn test_scientific() {
        let k = kinds("1e10");
        assert_eq!(k, vec![TokenKind::Number(1e10), TokenKind::Eof]);
    }

    #[test]
    fn test_keyword() {
        let k = kinds("100 km in m");
        assert!(k.contains(&TokenKind::Keyword(Keyword::In)));
    }

    #[test]
    fn test_comment() {
        let k = kinds("# hello");
        assert!(matches!(&k[0], TokenKind::Comment(s) if s == " hello"));
    }
}
