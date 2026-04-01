use crate::ast::*;
use crate::error::{Result, VenturiError};
use crate::lexer::{SpannedToken, Token};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn peek_spanned(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.pos)
    }

    fn current_line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map(|t| t.line)
            .unwrap_or(0)
    }

    fn advance(&mut self) -> &SpannedToken {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<&SpannedToken> {
        if self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(VenturiError::Parse {
                line: self.current_line(),
                msg: format!("Expected {:?}, got {:?}", expected, self.peek()),
            })
        }
    }

    fn skip_newlines(&mut self) {
        while self.peek() == &Token::Newline || self.peek() == &Token::Comment {
            self.advance();
        }
    }

    fn skip_comments_and_newlines(&mut self) {
        while matches!(
            self.peek(),
            Token::Newline | Token::Comment | Token::MetaComment(_)
        ) {
            self.advance();
        }
    }

    pub fn parse_file(&mut self) -> Result<VtFile> {
        // First token may be Shebang
        let kind = if let Token::Shebang(s) = self.peek().clone() {
            self.advance();
            if s.trim() == "plane" || s.trim().ends_with("plane") {
                NodeKind::Plane
            } else if s.trim() == "vortex" || s.trim().ends_with("vortex") {
                NodeKind::Vortex
            } else {
                NodeKind::Plane
            }
        } else {
            NodeKind::Plane
        };

        let mut vt = VtFile::new(kind);

        // Skip newlines after shebang
        self.skip_newlines();

        // Parse top-level declarations
        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::Eof => break,
                Token::VanDecl(van) => {
                    vt.van = Some(van.clone());
                    self.advance();
                }
                Token::MetaComment(content) => {
                    // Parse META: or key: value
                    let content = content.clone();
                    self.advance();
                    if content.starts_with("META:") {
                        // Just a marker
                    } else if content.contains(':') {
                        let parts: Vec<&str> = content.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            vt.meta.insert(
                                parts[0].trim().to_string(),
                                parts[1].trim().to_string(),
                            );
                        }
                    }
                }
                Token::Comment => {
                    self.advance();
                }
                Token::KwInput => {
                    let decl = self.parse_input_decl()?;
                    vt.inputs.push(decl);
                }
                Token::KwOutput => {
                    let decl = self.parse_output_decl()?;
                    vt.outputs.push(decl);
                }
                Token::KwUse => {
                    let use_decl = self.parse_use_chassis()?;
                    vt.uses.push(use_decl);
                }
                Token::KwPit => {
                    let pit = self.parse_pit_ref()?;
                    vt.pits.push(pit);
                }
                Token::KwFunc => {
                    let func = self.parse_func_def()?;
                    vt.func = Some(func);
                }
                Token::Ident(_) => {
                    // Could be a DAG wire: ident -> ident
                    if let Some(wire) = self.try_parse_dag_wire()? {
                        vt.dag_wires.push(wire);
                    } else {
                        // Unknown top-level statement, skip
                        self.advance();
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }

        Ok(vt)
    }

    fn parse_input_decl(&mut self) -> Result<InputDecl> {
        self.expect(&Token::KwInput)?;
        let name = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;

        let default = if self.peek() == &Token::Equals {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(InputDecl { name, ty, default })
    }

    fn parse_output_decl(&mut self) -> Result<OutputDecl> {
        self.expect(&Token::KwOutput)?;
        let name = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        Ok(OutputDecl { name, ty })
    }

    fn parse_use_chassis(&mut self) -> Result<UseChass> {
        self.expect(&Token::KwUse)?;
        self.expect(&Token::KwChassis)?;

        // Path is an ident (possibly with dots or slashes)
        let path = self.expect_ident_or_path()?;
        self.expect(&Token::KwAs)?;
        let alias = self.expect_ident()?;

        Ok(UseChass { path, alias })
    }

    fn parse_pit_ref(&mut self) -> Result<PitRef> {
        self.expect(&Token::KwPit)?;
        // Expect @ followed by URL
        self.expect(&Token::At)?;
        let url = self.expect_ident_or_path()?;
        Ok(PitRef {
            url: format!("@{}", url),
        })
    }

    fn parse_func_def(&mut self) -> Result<FuncDef> {
        self.expect(&Token::KwFunc)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;

        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let param = self.expect_ident()?;
            params.push(param);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::Colon)?;

        // Expect newline then indent
        self.skip_newlines();
        self.expect(&Token::Indent)?;

        let body = self.parse_block()?;

        Ok(FuncDef { name, params, body })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::Dedent | Token::Eof => {
                    if self.peek() == &Token::Dedent {
                        self.advance();
                    }
                    break;
                }
                _ => {
                    let stmt = self.parse_stmt()?;
                    stmts.push(stmt);
                }
            }
        }

        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.peek().clone() {
            Token::KwReturn => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(Stmt::Return(expr))
            }
            Token::KwTry => {
                self.parse_try_catch()
            }
            Token::KwLog => {
                self.advance();
                self.expect(&Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Stmt::Expr(Expr::Call("log".to_string(), vec![expr])))
            }
            Token::Ident(name) => {
                let name = name.clone();
                // Look ahead to determine if assignment or expression
                let next_pos = self.pos + 1;
                let next_tok = self.tokens.get(next_pos).map(|t| &t.token);
                if next_tok == Some(&Token::Equals) {
                    // Assignment
                    self.advance(); // consume ident
                    self.advance(); // consume =
                    let expr = self.parse_expr()?;
                    Ok(Stmt::Assign(name, expr))
                } else {
                    // Expression statement
                    let expr = self.parse_expr()?;
                    Ok(Stmt::Expr(expr))
                }
            }
            _ => {
                let expr = self.parse_expr()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_try_catch(&mut self) -> Result<Stmt> {
        self.expect(&Token::KwTry)?;
        self.expect(&Token::Colon)?;
        self.skip_newlines();
        self.expect(&Token::Indent)?;
        let body = self.parse_block()?;

        let mut catches = Vec::new();
        self.skip_newlines();

        while self.peek() == &Token::KwCatch {
            self.advance(); // consume 'catch'
            let error_type = self.expect_ident()?;
            self.expect(&Token::KwAs)?;
            let binding = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            self.skip_newlines();
            self.expect(&Token::Indent)?;
            let catch_body = self.parse_block()?;
            catches.push(CatchClause {
                error_type,
                binding,
                body: catch_body,
            });
            self.skip_newlines();
        }

        Ok(Stmt::TryCatch { body, catches })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        match self.peek().clone() {
            Token::ResultOk => {
                self.advance();
                self.expect(&Token::LParen)?;
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::ResultOk(Box::new(inner)))
            }
            Token::ResultErr => {
                self.advance();
                self.expect(&Token::LParen)?;
                let inner = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(Expr::ResultErr(Box::new(inner)))
            }
            Token::IntLit(n) => {
                let n = n;
                self.advance();
                Ok(Expr::IntLit(n))
            }
            Token::FloatLit(f) => {
                let f = f;
                self.advance();
                Ok(Expr::FloatLit(f))
            }
            Token::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::StrLit(s))
            }
            Token::BoolLit(b) => {
                let b = b;
                self.advance();
                Ok(Expr::BoolLit(b))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for function call
                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                        args.push(self.parse_expr()?);
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Expr::Call(name, args))
                } else if self.peek() == &Token::Dot {
                    self.advance();
                    let field = self.expect_ident()?;
                    Ok(Expr::FieldAccess(Box::new(Expr::Ident(name)), field))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            tok => Err(VenturiError::Parse {
                line: self.current_line(),
                msg: format!("Unexpected token in expression: {:?}", tok),
            }),
        }
    }

    fn parse_type(&mut self) -> Result<VtType> {
        match self.peek().clone() {
            Token::TypeName(name) => {
                let name = name.clone();
                self.advance();
                match name.as_str() {
                    "Int" => Ok(VtType::Int),
                    "Float" => Ok(VtType::Float),
                    "Bool" => Ok(VtType::Bool),
                    "String" => Ok(VtType::Str),
                    "DataFrame" => Ok(VtType::DataFrame),
                    other => Ok(VtType::Custom(other.to_string())),
                }
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(VtType::Custom(name))
            }
            tok => Err(VenturiError::Parse {
                line: self.current_line(),
                msg: format!("Expected type, got {:?}", tok),
            }),
        }
    }

    fn try_parse_dag_wire(&mut self) -> Result<Option<DagWire>> {
        // Check if pattern is: Ident Arrow Ident
        let from = if let Token::Ident(s) = self.peek().clone() {
            s.clone()
        } else {
            return Ok(None);
        };

        // Peek ahead
        let next_pos = self.pos + 1;
        let next_tok = self.tokens.get(next_pos).map(|t| &t.token);

        if next_tok != Some(&Token::Arrow) {
            return Ok(None);
        }

        self.advance(); // consume from ident
        self.advance(); // consume ->

        let to = self.expect_ident_or_path()?;

        Ok(Some(DagWire { from, to }))
    }

    fn expect_ident(&mut self) -> Result<String> {
        match self.peek().clone() {
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            tok => Err(VenturiError::Parse {
                line: self.current_line(),
                msg: format!("Expected identifier, got {:?}", tok),
            }),
        }
    }

    fn expect_ident_or_path(&mut self) -> Result<String> {
        // Accept idents, paths with dots/slashes
        match self.peek().clone() {
            Token::Ident(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            tok => Err(VenturiError::Parse {
                line: self.current_line(),
                msg: format!("Expected path/identifier, got {:?}", tok),
            }),
        }
    }
}

pub fn parse(tokens: Vec<SpannedToken>) -> Result<VtFile> {
    let mut parser = Parser::new(tokens);
    parser.parse_file()
}
