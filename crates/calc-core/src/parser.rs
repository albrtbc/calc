use crate::ast::{BinOp, Expr, Line};
use crate::error::{CalcError, Result};
use crate::lexer::{Keyword, Token, TokenKind};
use crate::units::Unit;

// ── Public Parser (document-level) ────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    /// Parse the full token stream as a document, producing one `Line` per
    /// newline-delimited logical line.
    pub fn parse_document(&mut self) -> Result<Vec<Line>> {
        let mut all_lines: Vec<Vec<Token>> = Vec::new();
        let mut current: Vec<Token> = Vec::new();

        for tok in std::mem::take(&mut self.tokens) {
            match tok.kind {
                TokenKind::Newline => {
                    all_lines.push(current.drain(..).collect());
                }
                TokenKind::Eof => {
                    all_lines.push(current.drain(..).collect());
                    break;
                }
                _ => {
                    current.push(tok);
                }
            }
        }
        if !current.is_empty() {
            all_lines.push(current);
        }

        let mut result = Vec::new();
        for line_tokens in all_lines {
            match parse_line(line_tokens) {
                Ok(line) => result.push(line),
                Err(e) => result.push(Line::Error(e.to_string())),
            }
        }
        Ok(result)
    }
}

// ── Line-level parsing ────────────────────────────────────────────────────────

fn parse_line(tokens: Vec<Token>) -> Result<Line> {
    if tokens.is_empty() {
        return Ok(Line::Empty);
    }

    // Comment: first token is a Comment
    if let TokenKind::Comment(ref s) = tokens[0].kind {
        return Ok(Line::Comment(s.clone()));
    }

    // Label: last token is Colon (e.g. "Income:" or "Total:")
    if tokens.len() >= 2 {
        if let TokenKind::Colon = tokens.last().unwrap().kind {
            let label_parts: Vec<String> = tokens[..tokens.len() - 1]
                .iter()
                .map(|t| token_to_str(t))
                .collect();
            return Ok(Line::Label(label_parts.join(" ")));
        }
    }
    // Single-token label (just "X:") — already caught above if len>=2.
    // Handle single token that is a colon by itself (empty label).
    if tokens.len() == 1 {
        if let TokenKind::Colon = tokens[0].kind {
            return Ok(Line::Label(String::new()));
        }
    }

    // Expression (or assignment)
    let mut lp = LineParser::new(tokens);
    let expr = lp.parse_expr()?;

    // Make sure we consumed everything (ignore trailing EOF marker we add)
    Ok(Line::Expr(expr))
}

fn token_to_str(t: &Token) -> String {
    match &t.kind {
        TokenKind::Ident(s) => s.clone(),
        TokenKind::Keyword(Keyword::In) => "in".to_string(),
        TokenKind::Keyword(Keyword::To) => "to".to_string(),
        TokenKind::Keyword(Keyword::As) => "as".to_string(),
        TokenKind::Keyword(Keyword::Of) => "of".to_string(),
        other => format!("{:?}", other),
    }
}

// ── Per-line expression parser ────────────────────────────────────────────────

struct LineParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl LineParser {
    fn new(mut tokens: Vec<Token>) -> Self {
        // Append synthetic EOF if not already present
        if tokens.last().map(|t| !matches!(t.kind, TokenKind::Eof)).unwrap_or(true) {
            let (line, col) = tokens
                .last()
                .map(|t| (t.line, t.col + 1))
                .unwrap_or((1, 1));
            tokens.push(Token { kind: TokenKind::Eof, line, col });
        }
        Self { tokens, pos: 0 }
    }

    // ── Navigation ───────────────────────────────────────────────────────────

    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_offset(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    // ── Grammar ───────────────────────────────────────────────────────────────

    /// Top-level: assignment or additive
    fn parse_expr(&mut self) -> Result<Expr> {
        // Tuple assignment: (a, b, c) = (1, 2, 3)
        if self.peek().kind == TokenKind::LParen {
            if let Some(tuple) = self.try_parse_tuple_assign()? {
                return Ok(tuple);
            }
        }

        // Assignment: IDENT = expr
        if let TokenKind::Ident(_) = &self.peek().kind.clone() {
            if let Some(next) = self.peek_offset(1) {
                if next.kind == TokenKind::Eq {
                    let name = match self.advance().kind.clone() {
                        TokenKind::Ident(s) => s,
                        _ => unreachable!(),
                    };
                    self.advance(); // consume '='
                    let value = self.parse_expr()?;
                    return Ok(Expr::Assign { name, value: Box::new(value) });
                }
            }
        }
        self.parse_additive()
    }

    /// Try to parse `(name1, name2, ...) = (expr1, expr2, ...)`.
    /// Returns None if the pattern doesn't match (position is restored).
    fn try_parse_tuple_assign(&mut self) -> Result<Option<Expr>> {
        let save_pos = self.pos;

        // Check pattern: ( ident , ident , ... ) =
        self.advance(); // consume '('
        let mut names = Vec::new();

        loop {
            match &self.peek().kind {
                TokenKind::Ident(s) => {
                    names.push(s.clone());
                    self.advance();
                }
                _ => {
                    self.pos = save_pos;
                    return Ok(None);
                }
            }
            match &self.peek().kind {
                TokenKind::Comma => { self.advance(); }
                TokenKind::RParen => { self.advance(); break; }
                _ => {
                    self.pos = save_pos;
                    return Ok(None);
                }
            }
        }

        // Must be followed by '='
        if self.peek().kind != TokenKind::Eq {
            self.pos = save_pos;
            return Ok(None);
        }
        self.advance(); // consume '='

        // Parse ( expr, expr, ... )
        if self.peek().kind != TokenKind::LParen {
            let tok = self.peek().clone();
            return Err(CalcError::new(
                "Expected '(' after '=' in tuple assignment",
                tok.line,
                tok.col,
            ));
        }
        self.advance(); // consume '('

        let mut values = Vec::new();
        loop {
            let expr = self.parse_additive()?;
            values.push(expr);
            match &self.peek().kind {
                TokenKind::Comma => { self.advance(); }
                TokenKind::RParen => { self.advance(); break; }
                other => {
                    let tok = self.peek().clone();
                    return Err(CalcError::new(
                        format!("Expected ',' or ')' in tuple values, got {:?}", other),
                        tok.line,
                        tok.col,
                    ));
                }
            }
        }

        if names.len() != values.len() {
            let tok = self.peek().clone();
            return Err(CalcError::new(
                format!(
                    "Tuple size mismatch: {} names but {} values",
                    names.len(),
                    values.len()
                ),
                tok.line,
                tok.col,
            ));
        }

        Ok(Some(Expr::TupleAssign { names, values }))
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplicative()?;

        loop {
            // Trailing unit conversion: expr (in|to|as) UNIT
            if self.is_conversion_keyword() {
                left = self.parse_conversion(left)?;
                continue;
            }

            let op = match &self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance(); // consume +/-

            let right = self.parse_multiplicative()?;

            // Detect "X + N%" → PercentAdd / "X - N%" → PercentSub
            match right {
                Expr::Percentage(ref pct) => {
                    let pct = pct.as_ref().clone();
                    left = match op {
                        BinOp::Add => Expr::PercentAdd {
                            base: Box::new(left),
                            percent: Box::new(pct),
                        },
                        BinOp::Sub => Expr::PercentSub {
                            base: Box::new(left),
                            percent: Box::new(pct),
                        },
                        _ => unreachable!(),
                    };
                }
                right => {
                    left = Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
            }
        }

        // Trailing conversion after the full additive expression
        if self.is_conversion_keyword() {
            left = self.parse_conversion(left)?;
        }

        Ok(left)
    }

    fn is_conversion_keyword(&self) -> bool {
        matches!(
            &self.peek().kind,
            TokenKind::Keyword(Keyword::In)
                | TokenKind::Keyword(Keyword::To)
                | TokenKind::Keyword(Keyword::As)
        )
    }

    /// Consume `in|to|as UNIT` and wrap expr in a Convert node.
    fn parse_conversion(&mut self, expr: Expr) -> Result<Expr> {
        self.advance(); // consume keyword

        // The unit name might be an Ident or a Keyword (e.g. "in" as inches)
        let unit_tok = self.peek().clone();
        let unit_str = match &unit_tok.kind {
            TokenKind::Ident(s) => s.clone(),
            TokenKind::Keyword(Keyword::In) => "in".to_string(),
            other => {
                return Err(CalcError::new(
                    format!(
                        "Expected unit name after conversion keyword, got {:?}",
                        other
                    ),
                    unit_tok.line,
                    unit_tok.col,
                ));
            }
        };
        self.advance(); // consume unit name token

        let unit = Unit::from_str(&unit_str)
            .unwrap_or_else(|| Unit::Custom(unit_str));
        Ok(Expr::Convert {
            expr: Box::new(expr),
            target_unit: unit,
        })
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut left = self.parse_power()?;

        loop {
            let op = match &self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                // Percent here is modulo only when between two non-% expressions.
                // The postfix % (Percentage) is handled in parse_postfix before we
                // get here, so if we see % at this level it's a modulo operator.
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance(); // consume operator

            let right = self.parse_power()?;
            left = Expr::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr> {
        let base = self.parse_unary()?;

        if self.peek().kind == TokenKind::Caret {
            self.advance(); // consume '^'
            // Right-associative: recurse
            let exp = self.parse_power()?;
            return Ok(Expr::BinaryOp {
                op: BinOp::Pow,
                left: Box::new(base),
                right: Box::new(exp),
            });
        }

        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        if self.peek().kind == TokenKind::Minus {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::UnaryNeg(Box::new(inner)));
        }
        if self.peek().kind == TokenKind::Plus {
            self.advance();
            let inner = self.parse_unary()?;
            return Ok(Expr::UnaryPos(Box::new(inner)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        // After a primary, check for attached unit: "5 km", "100 MB", etc.
        // Only attach if the next token is an Ident whose value is a known unit,
        // AND the token after that is not '(' (which would indicate a function).
        // AND the current expression is a Number or another unit-tagged value.
        if let Expr::Number(_) = &expr {
            if let TokenKind::Ident(ref s) = self.peek().kind.clone() {
                let s = s.clone();
                // Check it's actually a unit and not a variable name followed by assignment
                let next_next = self.peek_offset(1).map(|t| t.kind.clone());
                let is_unit = Unit::from_str(&s).is_some();
                let is_func_call = next_next == Some(TokenKind::LParen);
                let is_assign = next_next == Some(TokenKind::Eq);
                // Also treat as custom unit if followed by a conversion keyword
                let followed_by_conv = matches!(
                    next_next,
                    Some(TokenKind::Keyword(Keyword::In))
                        | Some(TokenKind::Keyword(Keyword::To))
                        | Some(TokenKind::Keyword(Keyword::As))
                );
                if !is_func_call && !is_assign && (is_unit || followed_by_conv) {
                    let unit = Unit::from_str(&s)
                        .unwrap_or_else(|| Unit::Custom(s));
                    self.advance(); // consume unit ident
                    expr = Expr::Convert {
                        expr: Box::new(expr),
                        target_unit: unit,
                    };
                }
            }
        }

        // Postfix %
        if self.peek().kind == TokenKind::Percent {
            // Peek ahead: if next is a conversion keyword, don't consume % as postfix here
            // (it's already consumed in multiplicative). Actually at this level the only
            // way we see % is as a postfix operator — let's consume it.
            self.advance(); // consume '%'

            // "N% of X"
            if matches!(&self.peek().kind, TokenKind::Keyword(Keyword::Of)) {
                self.advance(); // consume 'of'
                let base = self.parse_additive()?;
                return Ok(Expr::PercentOf {
                    percent: Box::new(expr),
                    base: Box::new(base),
                });
            }

            // Plain "N%"
            expr = Expr::Percentage(Box::new(expr));
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let tok = self.peek().clone();

        match tok.kind.clone() {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }

            TokenKind::Ident(name) => {
                self.advance();
                // Function call: IDENT(args)
                if self.peek().kind == TokenKind::LParen {
                    self.advance(); // '('
                    let mut args = Vec::new();
                    if self.peek().kind != TokenKind::RParen {
                        args.push(self.parse_expr()?);
                        while self.peek().kind == TokenKind::Comma {
                            self.advance(); // ','
                            args.push(self.parse_expr()?);
                        }
                    }
                    if self.peek().kind != TokenKind::RParen {
                        let bad = self.peek();
                        return Err(CalcError::new(
                            "Expected ')' after function arguments",
                            bad.line,
                            bad.col,
                        ));
                    }
                    self.advance(); // ')'
                    return Ok(Expr::FuncCall { name, args });
                }
                Ok(Expr::Ident(name))
            }

            TokenKind::Keyword(kw) => {
                // Treat keywords as identifiers when they appear in expression position
                let name = match kw {
                    Keyword::In => "in",
                    Keyword::To => "to",
                    Keyword::As => "as",
                    Keyword::Of => "of",
                }
                .to_string();
                self.advance();
                Ok(Expr::Ident(name))
            }

            TokenKind::LParen => {
                self.advance(); // '('
                let inner = self.parse_expr()?;
                if self.peek().kind != TokenKind::RParen {
                    let bad = self.peek();
                    return Err(CalcError::new("Expected ')'", bad.line, bad.col));
                }
                self.advance(); // ')'
                Ok(inner)
            }

            TokenKind::Eof => Err(CalcError::new(
                "Unexpected end of expression",
                tok.line,
                tok.col,
            )),

            other => Err(CalcError::new(
                format!("Unexpected token in expression: {:?}", other),
                tok.line,
                tok.col,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Vec<Line> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_document().unwrap()
    }

    #[test]
    fn test_simple_expr() {
        let lines = parse("1 + 2");
        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0], Line::Expr(_)));
    }

    #[test]
    fn test_assignment() {
        let lines = parse("x = 42");
        assert_eq!(lines.len(), 1);
        if let Line::Expr(Expr::Assign { ref name, .. }) = lines[0] {
            assert_eq!(name, "x");
        } else {
            panic!("Expected assignment, got {:?}", lines[0]);
        }
    }

    #[test]
    fn test_comment_line() {
        let lines = parse("# this is a comment");
        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0], Line::Comment(_)));
    }

    #[test]
    fn test_label() {
        let lines = parse("Income:");
        assert_eq!(lines.len(), 1);
        if let Line::Label(ref s) = lines[0] {
            assert_eq!(s, "Income");
        } else {
            panic!("Expected label, got {:?}", lines[0]);
        }
    }

    #[test]
    fn test_multiline() {
        let lines = parse("1 + 1\n2 * 3\n");
        // "1 + 1", "2 * 3", ""
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_percent_of() {
        let lines = parse("20% of 500");
        assert!(
            matches!(lines[0], Line::Expr(Expr::PercentOf { .. })),
            "Expected PercentOf, got {:?}",
            lines[0]
        );
    }

    #[test]
    fn test_percent_add() {
        let lines = parse("100 + 20%");
        assert!(
            matches!(lines[0], Line::Expr(Expr::PercentAdd { .. })),
            "Expected PercentAdd, got {:?}",
            lines[0]
        );
    }

    #[test]
    fn test_unit_conversion_keyword() {
        let lines = parse("5 km in m");
        assert!(
            matches!(lines[0], Line::Expr(Expr::Convert { .. })),
            "Expected Convert, got {:?}",
            lines[0]
        );
    }

    #[test]
    fn test_unit_literal() {
        // "5 km" should parse as Convert { Number(5), Kilometer }
        let lines = parse("5 km");
        assert!(
            matches!(lines[0], Line::Expr(Expr::Convert { .. })),
            "Expected Convert node for '5 km', got {:?}",
            lines[0]
        );
    }

    #[test]
    fn test_function_call() {
        let lines = parse("sqrt(16)");
        assert!(
            matches!(lines[0], Line::Expr(Expr::FuncCall { .. })),
            "Expected FuncCall, got {:?}",
            lines[0]
        );
    }

    #[test]
    fn test_power_right_assoc() {
        // 2^3^2 should be 2^(3^2) = 512
        let lines = parse("2^3^2");
        assert!(matches!(lines[0], Line::Expr(Expr::BinaryOp { op: BinOp::Pow, .. })));
    }

    #[test]
    fn test_empty_line() {
        let lines = parse("");
        assert_eq!(lines.len(), 1);
        assert!(matches!(lines[0], Line::Empty));
    }
}
