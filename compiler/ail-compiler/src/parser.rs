use crate::{
    Block, Declaration, Diagnostic, Effect, Expr, Field, FunctionDecl, Keyword, LetBinding,
    MatchArm, Parameter, ParameterType, RecordDecl, RecordFieldValue, SourceUnit, Span, Token,
    TokenKind, VariantCase, VariantDecl, lex,
};

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub unit: SourceUnit,
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn parse(source: &str) -> ParseResult {
    let tokens = lex(source);
    let significant = tokens
        .iter()
        .filter(|token| !token.kind.is_trivia())
        .cloned()
        .collect();
    let mut parser = Parser {
        tokens: significant,
        cursor: 0,
        diagnostics: Vec::new(),
    };
    let declarations = parser.parse_declarations();
    let unit = SourceUnit {
        declarations,
        span: Span::new(0, source.len()),
        tokens: tokens.clone(),
    };
    ParseResult {
        unit,
        tokens,
        diagnostics: parser.diagnostics,
    }
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();
        while !self.at(TokenKind::Eof) {
            let declaration = match self.current().kind {
                TokenKind::Keyword(Keyword::Record) => self.parse_record().map(Declaration::Record),
                TokenKind::Keyword(Keyword::Variant) => {
                    self.parse_variant().map(Declaration::Variant)
                }
                TokenKind::Keyword(Keyword::Fn) => self.parse_function().map(Declaration::Function),
                _ => {
                    self.report_expected("declaration");
                    self.advance();
                    None
                }
            };
            if let Some(declaration) = declaration {
                declarations.push(declaration);
            }
        }
        declarations
    }

    fn parse_record(&mut self) -> Option<RecordDecl> {
        let start = self.advance().span.start;
        let name = self.take_identifier()?;
        let identity = self.parse_identity()?;
        self.expect(TokenKind::LeftBrace, "{");
        let mut fields = Vec::new();

        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let field_start = self.current().span.start;
            let Some(field_name) = self.take_identifier() else {
                self.recover_until(&[TokenKind::Semicolon, TokenKind::RightBrace, TokenKind::Eof]);
                self.consume_if(TokenKind::Semicolon);
                continue;
            };
            let identity = self.parse_identity()?;

            if !self.consume_if(TokenKind::Colon) {
                let at = self.previous().span.end;
                let actual = self.current_actual();
                self.diagnostics
                    .push(Diagnostic::expected_token(Span::empty(at), ":", &actual));
                self.recover_until(&[TokenKind::Semicolon, TokenKind::RightBrace, TokenKind::Eof]);
                let end = if self.consume_if(TokenKind::Semicolon) {
                    self.previous().span.end
                } else {
                    self.current().span.start
                };
                fields.push(Field {
                    name: field_name,
                    identity,
                    ty: "<missing>".to_owned(),
                    span: Span::new(field_start, end),
                });
                continue;
            }

            let ty = self.take_identifier()?;
            self.expect(TokenKind::Semicolon, ";");
            fields.push(Field {
                name: field_name,
                identity,
                ty,
                span: Span::new(field_start, self.previous().span.end),
            });
        }
        self.expect(TokenKind::RightBrace, "}");
        Some(RecordDecl {
            name,
            identity,
            fields,
            span: Span::new(start, self.previous().span.end),
        })
    }

    fn parse_variant(&mut self) -> Option<VariantDecl> {
        let start = self.advance().span.start;
        let name = self.take_identifier()?;
        let identity = self.parse_identity()?;
        self.expect(TokenKind::LeftBrace, "{");
        let mut cases = Vec::new();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let case_start = self.current().span.start;
            let case_name = self.take_identifier()?;
            let identity = self.parse_identity()?;
            let payload = if self.consume_if(TokenKind::LeftParen) {
                let payload = self.take_identifier()?;
                self.expect(TokenKind::RightParen, ")");
                Some(payload)
            } else {
                None
            };
            self.expect(TokenKind::Semicolon, ";");
            cases.push(VariantCase {
                name: case_name,
                identity,
                payload,
                span: Span::new(case_start, self.previous().span.end),
            });
        }
        self.expect(TokenKind::RightBrace, "}");
        Some(VariantDecl {
            name,
            identity,
            cases,
            span: Span::new(start, self.previous().span.end),
        })
    }

    #[allow(clippy::option_option)]
    fn parse_identity(&mut self) -> Option<Option<String>> {
        if !self.at(TokenKind::Identifier) || self.current().text != "identity" {
            return Some(None);
        }
        self.advance();
        if !self.at(TokenKind::Text) {
            self.report_expected("identity string");
            return None;
        }
        let token = self.advance().clone();
        let identity = serde_json::from_str(&token.text).ok()?;
        Some(Some(identity))
    }

    fn parse_function(&mut self) -> Option<FunctionDecl> {
        let start = self.advance().span.start;
        let name = self.take_identifier()?;
        self.expect(TokenKind::LeftParen, "(");
        let parameters = self.parse_parameters()?;
        self.expect(TokenKind::RightParen, ")");
        self.expect(TokenKind::Arrow, "->");
        let result_type = self.take_identifier()?;
        let effects = if self.consume_if(TokenKind::Keyword(Keyword::Effects)) {
            self.parse_effects()?
        } else {
            Vec::new()
        };
        let body = self.parse_block()?;
        let end = body.span.end;
        Some(FunctionDecl {
            name,
            parameters,
            result_type,
            effects,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_parameters(&mut self) -> Option<Vec<Parameter>> {
        let mut parameters = Vec::new();
        if self.at(TokenKind::RightParen) {
            return Some(parameters);
        }
        loop {
            let start = self.current().span.start;
            let name = self.take_identifier()?;
            self.expect(TokenKind::Colon, ":");
            let ty = if self.consume_if(TokenKind::Keyword(Keyword::Capability)) {
                ParameterType::Capability(self.take_identifier()?)
            } else {
                ParameterType::Named(self.take_identifier()?)
            };
            parameters.push(Parameter {
                name,
                ty,
                span: Span::new(start, self.previous().span.end),
            });
            if !self.consume_if(TokenKind::Comma) {
                break;
            }
            if self.at(TokenKind::RightParen) {
                break;
            }
        }
        Some(parameters)
    }

    fn parse_effects(&mut self) -> Option<Vec<Effect>> {
        self.expect(TokenKind::LeftBrace, "{");
        let mut effects = Vec::new();
        if !self.at(TokenKind::RightBrace) {
            loop {
                let start = self.current().span.start;
                let receiver = self.take_identifier()?;
                self.expect(TokenKind::Dot, ".");
                let operation = self.take_identifier()?;
                effects.push(Effect {
                    receiver,
                    operation,
                    span: Span::new(start, self.previous().span.end),
                });
                if !self.consume_if(TokenKind::Comma) {
                    break;
                }
                if self.at(TokenKind::RightBrace) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBrace, "}");
        Some(effects)
    }

    fn parse_block(&mut self) -> Option<Block> {
        let start = self.current().span.start;
        self.expect(TokenKind::LeftBrace, "{");
        let mut bindings = Vec::new();
        while self.consume_if(TokenKind::Keyword(Keyword::Let)) {
            let binding_start = self.previous().span.start;
            let name = self.take_identifier()?;
            self.expect(TokenKind::Equal, "=");
            let value = self.parse_expression()?;
            self.expect(TokenKind::Semicolon, ";");
            bindings.push(LetBinding {
                name,
                value,
                span: Span::new(binding_start, self.previous().span.end),
            });
        }
        let tail = self.parse_expression()?;
        self.expect(TokenKind::RightBrace, "}");
        Some(Block {
            bindings,
            tail,
            span: Span::new(start, self.previous().span.end),
        })
    }

    fn parse_expression(&mut self) -> Option<Expr> {
        if self.at(TokenKind::Keyword(Keyword::If)) {
            return self.parse_if_expression();
        }
        if self.at(TokenKind::Keyword(Keyword::Match)) {
            return self.parse_match_expression();
        }
        let token = self.current().clone();
        let expression = match token.kind {
            TokenKind::Text => {
                self.advance();
                let value =
                    serde_json::from_str(&token.text).unwrap_or_else(|_| token.text.clone());
                Expr::Text {
                    value,
                    span: token.span,
                }
            }
            TokenKind::Integer => {
                self.advance();
                Expr::Integer {
                    spelling: token.text,
                    span: token.span,
                }
            }
            TokenKind::Identifier => {
                self.advance();
                let name = token.text;
                if self.consume_if(TokenKind::LeftBrace) {
                    return self.parse_record_expression(name, token.span.start);
                } else if self.consume_if(TokenKind::ColonColon) {
                    return self.parse_variant_expression(name, token.span.start);
                }
                Expr::Name {
                    name,
                    span: token.span,
                }
            }
            _ => {
                self.report_expected("expression");
                return None;
            }
        };
        self.parse_postfix_expression(expression)
    }

    fn parse_postfix_expression(&mut self, mut expression: Expr) -> Option<Expr> {
        while self.consume_if(TokenKind::Dot) {
            let field = self.take_identifier()?;
            if self.consume_if(TokenKind::LeftParen) {
                let start = expression.span().start;
                let Expr::Name { name: receiver, .. } = expression else {
                    self.report_expected("intrinsic or capability receiver");
                    return None;
                };
                let mut arguments = Vec::new();
                if !self.at(TokenKind::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.consume_if(TokenKind::Comma) {
                            break;
                        }
                        if self.at(TokenKind::RightParen) {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::RightParen, ")");
                expression = Expr::CapabilityCall {
                    receiver,
                    operation: field,
                    arguments,
                    span: Span::new(start, self.previous().span.end),
                };
            } else {
                let start = expression.span().start;
                expression = Expr::FieldAccess {
                    target: Box::new(expression),
                    field,
                    span: Span::new(start, self.previous().span.end),
                };
            }
        }
        Some(expression)
    }

    fn parse_if_expression(&mut self) -> Option<Expr> {
        let start = self.advance().span.start;
        let condition = Box::new(self.parse_expression_before_block()?);
        let then_branch = self.parse_block()?;
        self.expect(TokenKind::Keyword(Keyword::Else), "else");
        let else_branch = self.parse_block()?;
        let end = else_branch.span.end;
        Some(Expr::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span: Span::new(start, end),
        })
    }

    fn parse_match_expression(&mut self) -> Option<Expr> {
        let start = self.advance().span.start;
        let scrutinee = Box::new(self.parse_expression_before_block()?);
        self.expect(TokenKind::LeftBrace, "{");
        let mut arms = Vec::new();
        while !self.at(TokenKind::RightBrace) && !self.at(TokenKind::Eof) {
            let arm_start = self.current().span.start;
            let type_name = self.take_identifier()?;
            self.expect(TokenKind::ColonColon, "::");
            let case = self.take_identifier()?;
            let binding = if self.consume_if(TokenKind::LeftParen) {
                let binding = self.take_identifier()?;
                self.expect(TokenKind::RightParen, ")");
                Some(binding)
            } else {
                None
            };
            self.expect(TokenKind::FatArrow, "=>");
            let body = self.parse_block()?;
            let end = body.span.end;
            arms.push(MatchArm {
                type_name,
                case,
                binding,
                body,
                span: Span::new(arm_start, end),
            });
            if !self.consume_if(TokenKind::Comma) && !self.at(TokenKind::RightBrace) {
                self.expect(TokenKind::Comma, ",");
            }
        }
        self.expect(TokenKind::RightBrace, "}");
        Some(Expr::Match {
            scrutinee,
            arms,
            span: Span::new(start, self.previous().span.end),
        })
    }

    fn parse_expression_before_block(&mut self) -> Option<Expr> {
        if self.at(TokenKind::Identifier)
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|token| token.kind == TokenKind::LeftBrace)
        {
            let token = self.advance();
            Some(Expr::Name {
                name: token.text,
                span: token.span,
            })
        } else {
            self.parse_expression()
        }
    }

    fn parse_record_expression(&mut self, name: String, start: usize) -> Option<Expr> {
        let mut fields = Vec::new();
        if !self.at(TokenKind::RightBrace) {
            loop {
                let field_start = self.current().span.start;
                let field_name = self.take_identifier()?;
                self.expect(TokenKind::Colon, ":");
                let value = self.parse_expression()?;
                fields.push(RecordFieldValue {
                    name: field_name,
                    span: Span::new(field_start, value.span().end),
                    value,
                });
                if !self.consume_if(TokenKind::Comma) {
                    break;
                }
                if self.at(TokenKind::RightBrace) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBrace, "}");
        Some(Expr::Record {
            name,
            fields,
            span: Span::new(start, self.previous().span.end),
        })
    }

    fn parse_variant_expression(&mut self, type_name: String, start: usize) -> Option<Expr> {
        let case = self.take_identifier()?;
        let payload = if self.consume_if(TokenKind::LeftParen) {
            let payload = self.parse_expression()?;
            self.expect(TokenKind::RightParen, ")");
            Some(Box::new(payload))
        } else {
            None
        };
        Some(Expr::Variant {
            type_name,
            case,
            payload,
            span: Span::new(start, self.previous().span.end),
        })
    }

    fn take_identifier(&mut self) -> Option<String> {
        if self.at(TokenKind::Identifier) {
            Some(self.advance().text)
        } else {
            self.report_expected("Identifier");
            None
        }
    }

    fn expect(&mut self, kind: TokenKind, expected: &str) {
        if self.at(kind) {
            self.advance();
        } else {
            let at = self.previous().span.end;
            let actual = self.current_actual();
            self.diagnostics.push(Diagnostic::expected_token(
                Span::empty(at),
                expected,
                &actual,
            ));
        }
    }

    fn report_expected(&mut self, expected: &str) {
        let at = self.previous().span.end;
        let actual = self.current_actual();
        self.diagnostics.push(Diagnostic::expected_token(
            Span::empty(at),
            expected,
            &actual,
        ));
    }

    fn current_actual(&self) -> String {
        match self.current().kind {
            TokenKind::Identifier | TokenKind::Text | TokenKind::Integer => {
                self.current().text.clone()
            }
            _ => self.current().kind.description().to_owned(),
        }
    }

    fn recover_until(&mut self, kinds: &[TokenKind]) {
        while !kinds.iter().any(|kind| self.at(*kind)) {
            self.advance();
        }
    }

    fn consume_if(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn previous(&self) -> &Token {
        if self.cursor == 0 {
            &self.tokens[0]
        } else {
            &self.tokens[self.cursor - 1]
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.cursor += 1;
        }
        token
    }
}
