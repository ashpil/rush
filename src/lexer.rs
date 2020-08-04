use crate::helpers::Mode;
use std::cell::RefCell;
use std::iter::Peekable;
use std::rc::Rc;
use std::vec::IntoIter;

// Parent enum for tokens
#[derive(Debug, PartialEq)]
pub enum Token {
    Word(String),
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
    fn read_phrase(&mut self, c: char) -> Option<Token> {
        let mut phrase = {
            match c {
                '\\' => match self.next_char() {
                    Some('\n') => {
                        let _ = self.advance_line();
                        self.skip_whitespace();
                        String::new()
                    }
                    Some(c) => c.to_string(),
                    None => String::new(),
                },
                _ => c.to_string(),
            }
        };
        while let Some(c) = self.peek_char() {
            if *c == '\\' {
                self.next_char();
                if let Some(c) = self.next_char() {
                    phrase.push(c);
                }
            } else if is_forbidden(*c) || c.is_whitespace() {
                break;
            } else {
                phrase.push(self.next_char().unwrap());
            }
        }
        if phrase.is_empty() {
            None
        } else if c != '\\' {
            if let Ok(num) = phrase.parse::<u32>() {
                Some(Token::Integer(num))
            } else {
                Some(Token::Word(phrase))
            }
        } else {
            Some(Token::Word(phrase))
        }
    }

    // Of course, I still haven't added everything I'll need to yet
    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        match self.next_char() {
            Some(c) => match c {
                // Check whether it's two or one `|` and `&` here, as I think this is
                // the easiest place to fit that logic in
                '|' => {
                    if let Some('|') = self.peek_char() {
                        self.next_char();
                        Some(Token::Op(Op::Or))
                    } else {
                        Some(Token::Op(Op::Pipe))
                    }
                }
                '&' => {
                    if let Some('&') = self.peek_char() {
                        self.next_char();
                        Some(Token::Op(Op::And))
                    } else {
                        Some(Token::Op(Op::Ampersand))
                    }
                }
                '>' => Some(Token::Op(Op::More)),
                '<' => Some(Token::Op(Op::Less)),
                '!' => Some(Token::Op(Op::Bang)),
                '(' => Some(Token::Punct(Punct::LParen)),
                ')' => Some(Token::Punct(Punct::RParen)),
                '{' => Some(Token::Punct(Punct::LBracket)),
                '}' => Some(Token::Punct(Punct::RBracket)),
                '"' => {
                    let mut phrase = String::new();
                    loop {
                        match self.next_char() {
                            Some('"') => break,
                            Some(c) => phrase.push(c),
                            None => {
                                if let Err(()) = self.advance_line() {
                                    eprintln!("rush: expected '\"' but found EOF");
                                    return None
                                }
                            }
                        }
                    }
                    Some(Token::Word(phrase))
                }
                _ => self.read_phrase(c),
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
