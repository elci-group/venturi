use crate::error::{Result, VenturiError};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Shebang(String),
    VanDecl(String),
    MetaComment(String),
    Comment,
    // Keywords
    KwInput,
    KwOutput,
    KwFunc,
    KwTry,
    KwCatch,
    KwReturn,
    KwUse,
    KwChassis,
    KwPit,
    KwAs,
    KwLog,
    // Identifiers and literals
    Ident(String),
    TypeName(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    // Punctuation
    Colon,
    Equals,
    Comma,
    Dot,
    Arrow,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Plus,
    Minus,
    Star,
    Slash,
    // Indentation
    Indent,
    Dedent,
    Newline,
    // Special
    At,
    ResultOk,
    ResultErr,
    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    pending: Vec<SpannedToken>,
    at_line_start: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            input: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            pending: Vec::new(),
            at_line_start: true,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.input.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_spaces(&mut self) -> usize {
        let mut count = 0;
        while self.peek() == Some(' ') {
            self.advance();
            count += 1;
        }
        count
    }

    fn current_line(&self) -> usize {
        self.line
    }

    fn current_col(&self) -> usize {
        self.col
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>> {
        let mut tokens = Vec::new();

        loop {
            // Drain pending tokens first
            if !self.pending.is_empty() {
                let tok = self.pending.remove(0);
                let is_eof = tok.token == Token::Eof;
                tokens.push(tok);
                if is_eof {
                    break;
                }
                continue;
            }

            let tok = self.next_token()?;
            let is_eof = tok.token == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<SpannedToken> {
        // Handle indentation at start of line
        if self.at_line_start {
            self.at_line_start = false;
            let indent_level = self.skip_spaces();

            // Skip blank lines and comment-only lines for indentation purposes
            if self.peek() == Some('\n') || self.peek() == Some('\r') || self.peek().is_none() {
                // blank line - don't emit indent/dedent
            } else if self.peek() == Some('#') {
                // comment line - handle normally, no indent change
            } else {
                let current_indent = *self.indent_stack.last().unwrap_or(&0);
                if indent_level > current_indent {
                    self.indent_stack.push(indent_level);
                    let line = self.line;
                    let col = self.col;
                    return Ok(SpannedToken {
                        token: Token::Indent,
                        line,
                        col,
                    });
                } else if indent_level < current_indent {
                    // May need multiple Dedents
                    while *self.indent_stack.last().unwrap_or(&0) > indent_level {
                        self.indent_stack.pop();
                        let line = self.line;
                        let col = self.col;
                        self.pending.push(SpannedToken {
                            token: Token::Dedent,
                            line,
                            col,
                        });
                    }
                    if !self.pending.is_empty() {
                        let tok = self.pending.remove(0);
                        return Ok(tok);
                    }
                }
            }
        }

        // Skip horizontal whitespace (not newlines)
        while self.peek() == Some(' ') || self.peek() == Some('\t') {
            self.advance();
        }

        let line = self.current_line();
        let col = self.current_col();

        match self.peek() {
            None => {
                // Emit dedents for remaining indent levels
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    self.pending.push(SpannedToken {
                        token: Token::Dedent,
                        line,
                        col,
                    });
                }
                if !self.pending.is_empty() {
                    return Ok(self.pending.remove(0));
                }
                Ok(SpannedToken {
                    token: Token::Eof,
                    line,
                    col,
                })
            }

            Some('\n') | Some('\r') => {
                // consume newline
                if self.peek() == Some('\r') {
                    self.advance();
                }
                if self.peek() == Some('\n') {
                    self.advance();
                }
                self.at_line_start = true;
                Ok(SpannedToken {
                    token: Token::Newline,
                    line,
                    col,
                })
            }

            Some('#') => self.lex_comment(line, col),

            Some('"') | Some('\'') => self.lex_string(line, col),

            Some(c) if c.is_ascii_digit() => self.lex_number(line, col),

            Some(c) if c.is_alphabetic() || c == '_' => self.lex_ident_or_keyword(line, col),

            Some('@') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::At,
                    line,
                    col,
                })
            }

            Some(':') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Colon,
                    line,
                    col,
                })
            }

            Some('=') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Equals,
                    line,
                    col,
                })
            }

            Some(',') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Comma,
                    line,
                    col,
                })
            }

            Some('.') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Dot,
                    line,
                    col,
                })
            }

            Some('+') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Plus,
                    line,
                    col,
                })
            }

            Some('*') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Star,
                    line,
                    col,
                })
            }

            Some('/') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::Slash,
                    line,
                    col,
                })
            }

            Some('-') => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(SpannedToken {
                        token: Token::Arrow,
                        line,
                        col,
                    })
                } else {
                    Ok(SpannedToken {
                        token: Token::Minus,
                        line,
                        col,
                    })
                }
            }

            Some('(') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::LParen,
                    line,
                    col,
                })
            }

            Some(')') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::RParen,
                    line,
                    col,
                })
            }

            Some('[') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::LBracket,
                    line,
                    col,
                })
            }

            Some(']') => {
                self.advance();
                Ok(SpannedToken {
                    token: Token::RBracket,
                    line,
                    col,
                })
            }

            Some(c) => Err(VenturiError::Parse {
                line,
                msg: format!("Unexpected character: {:?}", c),
            }),
        }
    }

    fn lex_comment(&mut self, line: usize, col: usize) -> Result<SpannedToken> {
        // consume '#'
        self.advance();

        // Check for shebang: #!
        if self.pos == 2 && self.peek() == Some('!') {
            // Actually check if this is first line
            self.advance(); // consume '!'
            let mut rest = String::new();
            while self.peek().is_some() && self.peek() != Some('\n') && self.peek() != Some('\r') {
                rest.push(self.advance().unwrap());
            }
            return Ok(SpannedToken {
                token: Token::Shebang(rest.trim().to_string()),
                line,
                col,
            });
        }

        // Read rest of line
        let mut content = String::new();
        while self.peek().is_some() && self.peek() != Some('\n') && self.peek() != Some('\r') {
            content.push(self.advance().unwrap());
        }

        let content = content.trim().to_string();

        // Check for VAN declaration: VAN: @ident
        if content.starts_with(" VAN:") || content.starts_with("VAN:") {
            let van_part = content
                .trim_start_matches(" VAN:")
                .trim_start_matches("VAN:")
                .trim()
                .to_string();
            return Ok(SpannedToken {
                token: Token::VanDecl(van_part),
                line,
                col,
            });
        }

        // Check for META: or key: value meta comments
        if content.starts_with(" META:") || content.starts_with("META:") {
            return Ok(SpannedToken {
                token: Token::MetaComment(content.trim_start_matches(' ').to_string()),
                line,
                col,
            });
        }

        // Check for key: value pattern (meta)
        if content.starts_with(' ') {
            let trimmed = content.trim();
            if trimmed.contains(':') && !trimmed.starts_with('#') {
                return Ok(SpannedToken {
                    token: Token::MetaComment(trimmed.to_string()),
                    line,
                    col,
                });
            }
        }

        Ok(SpannedToken {
            token: Token::Comment,
            line,
            col,
        })
    }

    fn lex_string(&mut self, line: usize, col: usize) -> Result<SpannedToken> {
        let quote = self.advance().unwrap();
        let mut s = String::new();

        loop {
            match self.advance() {
                None => {
                    return Err(VenturiError::Parse {
                        line,
                        msg: "Unterminated string literal".to_string(),
                    })
                }
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some(c) => {
                            s.push('\\');
                            s.push(c);
                        }
                        None => {
                            return Err(VenturiError::Parse {
                                line,
                                msg: "Unterminated escape sequence".to_string(),
                            })
                        }
                    }
                }
                Some(c) if c == quote => break,
                Some(c) => s.push(c),
            }
        }

        Ok(SpannedToken {
            token: Token::StringLit(s),
            line,
            col,
        })
    }

    fn lex_number(&mut self, line: usize, col: usize) -> Result<SpannedToken> {
        let mut num_str = String::new();
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(self.advance().unwrap());
            } else if c == '.' && self.peek_ahead(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                is_float = true;
                num_str.push(self.advance().unwrap()); // '.'
            } else {
                break;
            }
        }

        if is_float {
            let f: f64 = num_str.parse().map_err(|_| VenturiError::Parse {
                line,
                msg: format!("Invalid float literal: {}", num_str),
            })?;
            Ok(SpannedToken {
                token: Token::FloatLit(f),
                line,
                col,
            })
        } else {
            let i: i64 = num_str.parse().map_err(|_| VenturiError::Parse {
                line,
                msg: format!("Invalid integer literal: {}", num_str),
            })?;
            Ok(SpannedToken {
                token: Token::IntLit(i),
                line,
                col,
            })
        }
    }

    fn lex_ident_or_keyword(&mut self, line: usize, col: usize) -> Result<SpannedToken> {
        let mut ident = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '-' {
                // Allow dots for things like Result.Ok, paths for pit URLs
                // But be careful about Arrow (->)
                if c == '-' && self.peek_ahead(1) == Some('>') {
                    break;
                }
                if c == '.' {
                    // Look ahead to see if it's Result.Ok / Result.Err
                    ident.push(self.advance().unwrap());
                    continue;
                }
                ident.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        // Check for special compound tokens
        if ident == "Result.Ok" {
            return Ok(SpannedToken {
                token: Token::ResultOk,
                line,
                col,
            });
        }
        if ident == "Result.Err" {
            return Ok(SpannedToken {
                token: Token::ResultErr,
                line,
                col,
            });
        }

        // Split on dot for field access - but we handle that in the parser
        // For now treat whole thing as one token if it doesn't match special patterns
        // Actually we should split on the first dot that isn't part of Result.Ok/Result.Err
        // Let's re-examine: if ident contains a dot but isn't Result.Ok/Err,
        // we need to handle it carefully. We'll just return as Ident and let parser handle

        let token = match ident.as_str() {
            "input" => Token::KwInput,
            "output" => Token::KwOutput,
            "func" => Token::KwFunc,
            "try" => Token::KwTry,
            "catch" => Token::KwCatch,
            "return" => Token::KwReturn,
            "use" => Token::KwUse,
            "chassis" => Token::KwChassis,
            "pit" => Token::KwPit,
            "as" => Token::KwAs,
            "log" => Token::KwLog,
            "true" => Token::BoolLit(true),
            "false" => Token::BoolLit(false),
            "Int" => Token::TypeName("Int".to_string()),
            "Float" => Token::TypeName("Float".to_string()),
            "Bool" => Token::TypeName("Bool".to_string()),
            "String" => Token::TypeName("String".to_string()),
            "DataFrame" => Token::TypeName("DataFrame".to_string()),
            _ => Token::Ident(ident),
        };

        Ok(SpannedToken { token, line, col })
    }
}

pub fn tokenize(source: &str) -> Result<Vec<SpannedToken>> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}
