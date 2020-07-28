use std::iter::Peekable;
use std::str::Chars;

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

#[derive(Debug)]
pub struct Lexer<'a> {
    text: Peekable<Chars<'a>>,
}

impl Lexer<'_> {
    pub fn new(text: &str) -> Lexer {
        Lexer {
            text: text.chars().peekable(),
        }
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.text.peek()
    }

    fn next_char(&mut self) -> Option<char> {
        self.text.next()
    }

    fn skip_whitespace(&mut self) {
        let mut next = self.peek_char(); // Is making this mutable better than
        while next.is_some() && next.unwrap().is_whitespace() {
            self.next_char();
            next = self.peek_char(); // doing `let next = self.peek_char()` (shadowing) here?
        }
    }

    // Reads a string of consecutive characters, then figures out if they're numbers of letters
    fn read_phrase(&mut self, c: char) -> Option<Token> {
        let mut phrase = c.to_string();
        while let Some(c) = self.peek_char() {
            if is_forbidden(c) || c.is_whitespace() {
                break;
            } else {
                phrase.push(self.next_char().unwrap());
            }
        }
        if let Ok(num) = phrase.parse::<u32>() {
            Some(Token::Integer(num))
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
                '!' => Some(Token::Op(Op::Bang)),
                '(' => Some(Token::Punct(Punct::LParen)),
                ')' => Some(Token::Punct(Punct::RParen)),
                '{' => Some(Token::Punct(Punct::LBracket)),
                '}' => Some(Token::Punct(Punct::RBracket)),
                _ => self.read_phrase(c),
            },
            None => None,
        }
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
         self.next_token()
    }
}

fn is_forbidden(c: &char) -> bool {
    matches!(*c, '&' | '!' | '|')
}

// TODO: More tests
#[cfg(test)]
mod lexer_tests {
    use super::{Lexer, Op, Token};

    #[test]
    fn test_lexer() {
        let mut lexer = Lexer::new("exa -1 | grep cargo");
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
