// The Lexer does the work required before the AST can be built
// in the parser

use crate::helpers::Shell;
use std::cell::RefCell;
use std::iter::Peekable;
use std::rc::Rc;
use std::vec::IntoIter;

use self::Expand::*;

// Parent enum for tokens
#[derive(Debug, PartialEq)]
pub enum Token {
    Word(Vec<Expand>),
    Integer(u32),
    Assign(String, Vec<Expand>),
    Op(Op),
    Punct(Punct),
}

#[derive(Debug, PartialEq)]
pub enum Expand {
    Literal(String),
    Var(String),
    Brace(String, Vec<Expand>),
    Tilde(String),
}

impl Expand {
    fn get_name(self) -> String {
        match self {
            Literal(s) => s,
            Var(s) => s,
            Brace(s, _) => s,
            Tilde(s) => s,
        }
    }
}

// Operators
#[derive(Debug, PartialEq)]
pub enum Op {
    Pipe,
    Ampersand,
    Bang,
    Or,
    And,
    Less,
    More,
}

// Punctuation
#[derive(Debug, PartialEq)]
pub enum Punct {
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
}

// This representation makes it's functions very nice and easy,
// but I'm not convinced that this is the most efficient/clean
// the struct itself can be
pub struct Lexer {
    shell: Rc<RefCell<Shell>>,
    line: Peekable<IntoIter<char>>,
}

impl Lexer {
    pub fn new(line: &str, shell: Rc<RefCell<Shell>>) -> Lexer {
        Lexer {
            shell,
            line: line.chars().collect::<Vec<_>>().into_iter().peekable(),
        }
    }

    fn advance_line(&mut self) -> Result<(), String> {
        if let Some(s) = self.shell.borrow_mut().next_prompt("> ") {
            self.line = s.chars().collect::<Vec<_>>().into_iter().peekable();
            Ok(())
        } else {
            Err(String::from("expected more input but found one"))
        }
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.line.peek()
    }

    fn next_char(&mut self) -> Option<char> {
        self.line.next()
    }

    fn skip_whitespace(&mut self) {
        let mut next = self.peek_char();
        while next.is_some() && next.unwrap().is_whitespace() {
            self.next_char();
            next = self.peek_char();
        }
    }

    fn read_words(&mut self) -> Result<Vec<Expand>, String> {
        let mut words = Vec::new();
        while let Some(c) = self.peek_char() {
            match c {
                '$' => {
                    self.next_char();
                    words.push(Var(self.read_literal()?));
                }
                '~' => {
                    self.next_char();
                    words.push(Tilde(self.read_literal()?));
                }
                '"' => {
                    self.next_char();
                    let mut phrase = String::new();
                    loop {
                        match self.next_char() {
                            Some('"') => break,
                            Some('\\') => match self.next_char() {
                                Some('\n') => self.advance_line()?,
                                Some(c) => phrase.push(c),
                                None => (),
                            },
                            Some(c) => phrase.push(c),
                            None => self.advance_line()?,
                        }
                    }
                    words.push(Literal(phrase));
                }
                c if is_forbidden(*c) || c.is_whitespace() => break,
                _ => words.push(Literal(self.read_literal()?)),
            }
        }
        Ok(words)
    }

    fn read_literal(&mut self) -> Result<String, String> {
        let mut phrase = String::new();
        while let Some(c) = self.peek_char() {
            match c {
                '\\' => {
                    self.next_char();
                    match self.next_char() {
                        Some('\n') => self.advance_line()?,
                        Some(c) => phrase.push(c),
                        None => break,
                    }
                }
                '=' => {
                    phrase.push(self.next_char().unwrap());
                    break
                }
                c if is_forbidden(*c) || c.is_whitespace() => break,
                _ => phrase.push(self.next_char().unwrap()),
            }
        }
        Ok(phrase)
    }

    // Of course, I still haven't added everything I'll need to yet
    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('|') => {
                self.next_char();
                if let Some('|') = self.peek_char() {
                    self.next_char();
                    Some(Token::Op(Op::Or))
                } else {
                    Some(Token::Op(Op::Pipe))
                }
            }
            Some('&') => {
                self.next_char();
                if let Some('&') = self.peek_char() {
                    self.next_char();
                    Some(Token::Op(Op::And))
                } else {
                    Some(Token::Op(Op::Ampersand))
                }
            }
            Some('>') => {
                self.next_char();
                Some(Token::Op(Op::More))
            }
            Some('<') => {
                self.next_char();
                Some(Token::Op(Op::Less))
            }
            Some('!') => {
                self.next_char();
                Some(Token::Op(Op::Bang))
            }
            Some('(') => {
                self.next_char();
                Some(Token::Punct(Punct::LParen))
            }
            Some(')') => {
                self.next_char();
                Some(Token::Punct(Punct::RParen))
            }
            Some(_) => {
                match self.read_words() {
                    Ok(w) => {
                        match &w[..] {
                            [Literal(s)] => {
                                if let Ok(num) = s.parse::<u32>() {
                                    Some(Token::Integer(num))
                                } else {
                                    Some(Token::Word(w))
                                }
                            }
                            [Literal(s), ..] if s.ends_with('=') => {
                                let mut iter = w.into_iter();
                                let mut name = iter.next().unwrap().get_name();
                                name.pop();
                                Some(Token::Assign(name, iter.collect()))
                            }
                            _ => Some(Token::Word(w)),
                        }
                    }
                    Err(e) => {
                        eprintln!("rush: {}", e);
                        None
                    }
                }
            }
            None => None,
        }
    }
}

impl Iterator for Lexer {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        self.next_token()
    }
}

fn is_forbidden(c: char) -> bool {
    matches!(c, '&' | '!' | '|' | '<' | '>' | '$')
}

// TODO: More tests
#[cfg(test)]
mod lexer_tests {
    use super::{Lexer, Op, Token};
    use crate::helpers::Shell;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_lexer() {
        let shell = Rc::new(RefCell::new(Shell::new(None)));
        let mut lexer = Lexer::new("exa -1 | grep cargo", Rc::clone(&shell));
        let expected = [
            Token::Word("exa".to_string()),
            Token::Word("-1".to_string()),
            Token::Op(Op::Pipe),
            Token::Word("grep".to_string()),
            Token::Word("cargo".to_string()),
        ];
        for token in &expected {
            assert_eq!(*token, lexer.next().unwrap())
        }
    }
}
