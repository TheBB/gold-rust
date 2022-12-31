use std::fmt::Display;
use std::iter::Iterator;
use regex::Regex;

use nom::InputLength;

use crate::error::{Location, Tagged, SyntaxError, Syntax, SyntaxElement};
use crate::traits::Taggable;


pub(crate) type LexResult<'a, T> = Result<(Lexer<'a>, T), SyntaxError>;


#[derive(Debug, PartialEq, Copy, Clone)]
pub enum TokenType {
    Asterisk,
    Caret,
    CloseBrace,
    CloseBracePipe,
    CloseBracket,
    CloseParen,
    Colon,
    Comma,
    Dollar,
    Dot,
    DoubleColon,
    DoubleEq,
    DoubleSlash,
    DoubleQuote,
    Ellipsis,
    Eq,
    ExclamEq,
    Greater,
    GreaterEq,
    Less,
    LessEq,
    Minus,
    OpenBrace,
    OpenBracePipe,
    OpenBracket,
    OpenParen,
    Pipe,
    Plus,
    SemiColon,
    Slash,

    Name,
    Float,
    Integer,
    StringLit,
    MultiString,
}


impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Asterisk => "'*'",
            Self::Caret => "'^'",
            Self::CloseBrace => "'}'",
            Self::CloseBracePipe => "'|}'",
            Self::CloseBracket => "']'",
            Self::CloseParen => "')'",
            Self::Colon => "':'",
            Self::Comma => "','",
            Self::Dollar => "'$'",
            Self::Dot => "'.'",
            Self::DoubleColon => "'::'",
            Self::DoubleEq => "'=='",
            Self::DoubleSlash => "'//'",
            Self::DoubleQuote => "'\"'",
            Self::Ellipsis => "'...'",
            Self::Eq => "'='",
            Self::ExclamEq => "'!='",
            Self::Greater => "'>'",
            Self::GreaterEq => "'>='",
            Self::Less => "'<'",
            Self::LessEq => "'<='",
            Self::Minus => "'-'",
            Self::OpenBrace => "'{'",
            Self::OpenBracePipe => "'{|'",
            Self::OpenBracket => "'['",
            Self::OpenParen => "'('",
            Self::Pipe => "'|'",
            Self::Plus => "'+'",
            Self::SemiColon => "';'",
            Self::Slash => "'/'",
            Self::Name => "name",
            Self::Float => "float",
            Self::Integer => "int",
            Self::StringLit => "string literal",
            Self::MultiString => "multi-line string literal",
        })
    }
}


#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) struct Token<'a> {
    pub kind: TokenType,
    pub span: &'a str,
}


#[derive(Clone, Copy, Debug)]
pub(crate) struct Lexer<'a> {
    pub code: &'a str,
    offset: usize,
    line: u32,
    col: u32,
}


lazy_static! {
    static ref WHITESPACE: Regex = Regex::new(r"^[^\S\n]*").unwrap();
    static ref NAME: Regex = Regex::new("^[[:alpha:]_][^\\s'\"{}()\\[\\]/+*\\-;:,.=#\\|]*").unwrap();
    static ref KEY: Regex = Regex::new("^[^\\s'\"{}()\\[\\]:]+").unwrap();
    static ref FLOAT_A: Regex = Regex::new(r"^[[:digit:]][[:digit:]_]*\.[[:digit:]_]*(?:(?:e|E)(?:\+|-)?[[:digit:]][[:digit:]_]*)?").unwrap();
    static ref FLOAT_B: Regex = Regex::new(r"^\.[[:digit:]][[:digit:]_]*(?:(?:e|E)[[:digit:]][[:digit:]_]*)?").unwrap();
    static ref FLOAT_C: Regex = Regex::new(r"^[[:digit:]][[:digit:]_]*(?:e|E)(?:\+|-)?[[:digit:]][[:digit:]_]*").unwrap();
    static ref DIGITS: Regex = Regex::new("^[[:digit:]][[:digit:]_]*").unwrap();
}


impl<'a> Lexer<'a> {
    pub fn new(code: &'a str) -> Lexer<'a> {
        Lexer {
            code,
            offset: 0,
            line: 1,
            col: 0,
        }
    }

    pub fn loc(&self) -> Location {
        Location {
            offset: self.offset,
            line: self.line,
            column: self.col as u32,
            length: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.code.chars().next()
    }

    fn satisfies_at(&self, i: usize, f: impl FnOnce(char) -> bool) -> bool {
        self.code.chars().nth(i).is_some_and(f)
    }

    fn skip(self, offset: usize, delta_line: u32) -> Self {
        Lexer {
            code: &self.code[offset..],
            offset: self.offset + offset,
            line: self.line + delta_line,
            col: if delta_line > 0 { 0 } else { self.col + offset as u32 }
        }
    }

    fn skip_tag(self, offset: usize, delta_line: u32, kind: TokenType) -> LexResult<'a, Tagged<Token<'a>>>
    {
        let code = self.code[..offset].tag(Location {
            offset: self.offset,
            column: self.col,
            line: self.line,
            length: offset,
        });

        Ok((self.skip(offset, delta_line), code.map(|span| Token { kind, span })))
    }

    fn traverse(self, regex: &'a Regex, element: SyntaxElement, kind: TokenType) -> LexResult<'a, Tagged<Token<'a>>> {
        regex.find(self.code).map(|m| {
            let lex = self.skip(m.start(), 0);
            lex.skip_tag(m.end() - m.start(), 0, kind).unwrap()
        }).ok_or_else(
            || SyntaxError(self.loc(), Some(Syntax::from(element)))
        )
    }

    pub fn skip_whitespace(mut self) -> Self {
        loop {
            self = self.skip_indent();

            match self.peek() {
                Some('\n') => {
                    self = self.skip(1, 1);
                    continue;
                },
                Some('#') => {
                    let end = self.code.find('\n').unwrap_or(self.code.len() - 1);
                    self = self.skip(end + 1, 1);
                },
                _ => {
                    break;
                },
            }
        }

        self
    }

    fn skip_indent(self) -> Self {
        // The WHITESPACE regex cannot fail to match, so unwrapping is safe
        WHITESPACE.find(self.code).map(|m| self.skip(m.end(), 0)).unwrap()
    }

    fn next_number(self) -> LexResult<'a, Tagged<Token<'a>>> {
        self.traverse(&FLOAT_A, SyntaxElement::Number, TokenType::Float)
        .or_else(|_| self.traverse(&FLOAT_B, SyntaxElement::Number, TokenType::Float))
        .or_else(|_| self.traverse(&FLOAT_C, SyntaxElement::Number, TokenType::Float))
        .or_else(|_| self.traverse(&DIGITS, SyntaxElement::Number, TokenType::Integer))
    }

    fn next_name(self, regex: &'a Regex) -> LexResult<'a, Tagged<Token<'a>>> {
        self.traverse(regex, SyntaxElement::Identifier, TokenType::Name)
    }

    pub fn next_token(mut self) -> LexResult<'a, Tagged<Token<'a>>> {
        self = self.skip_whitespace();

        let s = match self.peek() {
            Some('a'..='z') | Some('A'..='Z') | Some('_') => self.next_name(&NAME),

            Some(x) if x.is_ascii_digit() => self.next_number(),

            Some('.') if self.satisfies_at(1, |x| x.is_ascii_digit()) => self.next_number(),
            Some('.') if self.satisfies_at(1, |x| x == '.') && self.satisfies_at(2, |x| x == '.') => self.skip_tag(3, 0, TokenType::Ellipsis),
            Some('.') => self.skip_tag(1, 0, TokenType::Dot),

            Some(':') if self.satisfies_at(1, |x| x == ':') => self.skip_tag(2, 0, TokenType::DoubleColon),
            Some(':') => self.skip_tag(1, 0, TokenType::Colon),

            Some('"') => self.skip_tag(1, 0, TokenType::DoubleQuote),
            Some('{') if self.satisfies_at(1, |x| x == '|') => self.skip_tag(2, 0, TokenType::OpenBracePipe),
            Some('{') => self.skip_tag(1, 0, TokenType::OpenBrace),
            Some('|') if self.satisfies_at(1, |x| x == '}') => self.skip_tag(2, 0, TokenType::CloseBracePipe),
            Some('}') => self.skip_tag(1, 0, TokenType::CloseBrace),
            Some('[') => self.skip_tag(1, 0, TokenType::OpenBracket),
            Some(']') => self.skip_tag(1, 0, TokenType::CloseBracket),
            Some('(') => self.skip_tag(1, 0, TokenType::OpenParen),
            Some(')') => self.skip_tag(1, 0, TokenType::CloseParen),
            Some(',') => self.skip_tag(1, 0, TokenType::Comma),
            Some('+') => self.skip_tag(1, 0, TokenType::Plus),
            Some('-') => self.skip_tag(1, 0, TokenType::Minus),
            Some('/') if self.satisfies_at(1, |x| x == '/') => self.skip_tag(2, 0, TokenType::DoubleSlash),
            Some('/') => self.skip_tag(1, 0, TokenType::Slash),
            Some('*') => self.skip_tag(1, 0, TokenType::Asterisk),
            Some('^') => self.skip_tag(1, 0, TokenType::Caret),
            Some('<') if self.satisfies_at(1, |x| x == '=') => self.skip_tag(2, 0, TokenType::LessEq),
            Some('<') => self.skip_tag(1, 0, TokenType::Less),
            Some('>') if self.satisfies_at(1, |x| x == '=') => self.skip_tag(2, 0, TokenType::GreaterEq),
            Some('>') => self.skip_tag(1, 0, TokenType::Greater),
            Some('=') if self.satisfies_at(1, |x| x == '=') => self.skip_tag(2, 0, TokenType::DoubleEq),
            Some('=') => self.skip_tag(1, 0, TokenType::Eq),
            Some('!') if self.satisfies_at(1, |x| x == '=') => self.skip_tag(2, 0, TokenType::ExclamEq),
            Some('|') => self.skip_tag(1, 0, TokenType::Pipe),
            Some(';') => self.skip_tag(1, 0, TokenType::SemiColon),

            Some(c) => Err(SyntaxError(self.loc(), Some(Syntax::UnexpectedChar(c)))),
            None => Err(SyntaxError(self.loc(), Some(Syntax::UnexpectedEof))),
        };

        // println!("returning token: {:?}", s.as_ref().map(|x| x.1));

        s
    }

    pub fn next_key(mut self) -> LexResult<'a, Tagged<Token<'a>>> {
        self = self.skip_whitespace();

        match self.peek() {
            Some('}') => self.skip_tag(1, 0, TokenType::CloseBrace),
            Some('$') => self.skip_tag(1, 0, TokenType::Dollar),
            Some('"') => self.skip_tag(1, 0, TokenType::DoubleQuote),
            Some(':') if self.satisfies_at(1, |x| x == ':') => self.skip_tag(2, 0, TokenType::DoubleColon),
            Some(':') => self.skip_tag(1, 0, TokenType::Colon),
            Some('.') if self.satisfies_at(1, |x| x == '.') && self.satisfies_at(2, |x| x == '.') => self.skip_tag(3, 0, TokenType::Ellipsis),
            Some(_) => self.next_name(&KEY),
            None => Err(SyntaxError(self.loc(), Some(Syntax::UnexpectedEof))),
        }
    }

    pub fn next_multistring(mut self, col: u32) -> LexResult<'a, Tagged<Token<'a>>> {
        let orig = self;

        let end = self.code.find('\n').unwrap_or(self.code.len() - 1);
        self = self.skip(end + 1, 1);

        loop {
            let skipped = self.skip_indent();
            if skipped.col <= col {
                break;
            }

            self = skipped;

            let end = self.code.find('\n').unwrap_or(self.code.len() - 1);
            self = self.skip(end + 1, 1);
        }

        let tok = Token {
            kind: TokenType::MultiString,
            span: &orig.code[..(self.offset - orig.offset)],
        }.tag(Location {
            offset: orig.offset,
            line: orig.line,
            column: orig.col,
            length: self.offset - orig.offset,
        });

        Ok((self, tok))
    }

    pub fn next_string(self) -> LexResult<'a, Tagged<Token<'a>>> {
        match self.peek() {
            None => Err(SyntaxError(self.loc(), Some(Syntax::UnexpectedEof))),

            Some('"') => self.skip_tag(1, 0, TokenType::DoubleQuote),
            Some('$') => self.skip_tag(1, 0, TokenType::Dollar),
            Some('\n') => Err(SyntaxError(self.loc(), Some(Syntax::UnexpectedChar('\n')))),

            _ => {
                let mut it = self.code.char_indices();
                loop {
                    match it.next() {
                        Some((end, '"' | '$' | '\n')) => {
                            return self.skip_tag(end, 0, TokenType::StringLit);
                        }

                        Some((end, '\\')) => {
                            let c = it.next();
                            if let Some((_, '"' | '\\' | '$')) = c {
                                continue;
                            } else if let Some((_, cc)) = c {
                                let lex = self.skip(end + 1, 0);
                                return Err(SyntaxError(lex.loc(), Some(Syntax::UnexpectedChar(cc))));
                            }
                            continue;
                        }

                        None => {
                            return self.skip_tag(self.code.len(), 0, TokenType::StringLit);
                        }

                        _ => { continue; }
                    }
                }
            }
        }
    }
}

impl<'a> InputLength for Lexer<'a> {
    fn input_len(&self) -> usize {
        self.code.len()
    }
}
