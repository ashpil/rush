use crate::helpers::Mode;
use std::cell::RefCell;
use std::iter::Peekable;
use std::rc::Rc;
use std::vec::IntoIter;

// Parent enum for tokens
#[derive(Debug, PartialEq)]
pub enum Token {
    Word(String),
    Tilde(String),
    Integer(u32),
    Op(Op),
    Punct(Punct),
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
    mode: Rc<RefCell<Mode>>,
    line: Peekable<IntoIter<char>>,
}

impl Lexer {
    pub fn new(line: &str, mode: Rc<RefCell<Mode>>) -> Lexer {
        Lexer {
            mode,
            line: line.chars().collect::<Vec<_>>().into_iter().peekable(),
        }
    }

    fn advance_line(&mut self) -> Result<(), ()> {
        if let Some(s) = self.mode.borrow_mut().next_prompt("> ") {
            self.line = s.chars().collect::<Vec<_>>().into_iter().peekable();
            Ok(())
        } else {
            Err(())
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

    // Reads a string of consecutive characters, then figures out if they're numbers of letters
    fn read_phrase(&mut self) -> Result<String, String> {
        let mut phrase = String::new();
        while let Some(c) = self.peek_char() {
            match c {
                '\\' => {
                    self.next_char();
                    match self.next_char() {
                        Some('\n') => {
                            let _ = self.advance_line();
                            self.skip_whitespace();
                        },
                        Some(c) => phrase.push(c),
                        None => (),
                    }
                },
                '"' => {
                    self.next_char();
                    loop {
                        match self.next_char() {
                            Some('"') => break,
                            Some('\\') => {
                                match self.next_char() {
                                    Some('\n') => {
                                        let _ = self.advance_line();
                                        self.skip_whitespace();
                                    },
                                    Some(c) => phrase.push(c),
                                    None => (),
                                }
                            },
                            Some(c) => phrase.push(c),
                            None => {
                                if let Err(()) = self.advance_line() {
                                    return Err(String::from("expected endquote but found EOF"));
                                }
                            }
                        }
                    }
                },
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
            },
            Some('&') => {
                self.next_char();
                if let Some('&') = self.peek_char() {
                    self.next_char();
                    Some(Token::Op(Op::And))
                } else {
                    Some(Token::Op(Op::Ampersand))
                }
            },
            Some('>') => {
                self.next_char();
                Some(Token::Op(Op::More))
            },
            Some('<') => {
                self.next_char();
                Some(Token::Op(Op::Less))
            },
            Some('!') => {
                self.next_char();
                Some(Token::Op(Op::Bang))
            },
            Some('(') => {
                self.next_char();
                Some(Token::Punct(Punct::LParen))
            },
            Some(')') => {
                self.next_char();
                Some(Token::Punct(Punct::RParen))
            },
            Some('{') => {
                self.next_char();
                Some(Token::Punct(Punct::LBracket))
            },
            Some('}') => {
                self.next_char();
                Some(Token::Punct(Punct::RBracket))
            },
            Some('~') => {
                self.next_char();
                match self.read_phrase() {
                    Ok(s) => Some(Token::Tilde(s)),
                    Err(e) => {
                        eprintln!("rush: {}", e);
                        None
                    },
                }
            },
            Some(c) => {
                let c = *c;
                match self.read_phrase() {
                    Ok(s) => {
                        if s.is_empty() {
                            None
                        } else if c == '\\' {
                            Some(Token::Word(s))
                        } else if let Ok(num) = s.parse::<u32>() {
                            Some(Token::Integer(num))
                        } else {
                            Some(Token::Word(s))
                        }
                    },
                    Err(e) => {
                        eprintln!("rush: {}", e);
                        None
                    },
                }
            },
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
    matches!(c, '&' | '!' | '|' | '<' | '>')
}

// TODO: More tests
#[cfg(test)]
mod lexer_tests {
    use super::{Lexer, Op, Token};
    use crate::helpers::Mode;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_lexer() {
        let mode = Rc::new(RefCell::new(Mode::new(None)));
        let mut lexer = Lexer::new("exa -1 | grep cargo", Rc::clone(&mode));
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
