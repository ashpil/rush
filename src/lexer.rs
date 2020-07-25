use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq)]
pub enum Token {
    Word(String),
    Integer(u32),
    Op(Op),
    Punct(Punct),
}

#[derive(Debug, PartialEq)]
pub enum Op {
    Pipe,
    Ampersand,
}

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
        loop {
            let next = self.peek_char();
            if !(next.is_some() && next.unwrap().is_whitespace()) {
                break;
            }
            self.next_char();
        }
    }

    fn read_phrase(&mut self, c: char) -> Option<Token> {
        let mut phrase = c.to_string();
        if is_name(&c) {
            loop {
                let next = self.peek_char();
                if !(next.is_some() && is_name(next.unwrap())) {
                    break;
                }
                phrase.push(self.next_char().unwrap());
            }
            Some(Token::Word(phrase))
        } else if c.is_digit(10) {
            loop {
                let next = self.peek_char();
                if !(next.is_some() && next.unwrap().is_digit(10)) {
                    break;
                }
                phrase.push(self.next_char().unwrap());
            }
            Some(Token::Integer(phrase.parse::<u32>().unwrap()))
        } else {
            None
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        match self.next_char() {
            Some(c) => match c {
                '|' => Some(Token::Op(Op::Pipe)),
                '&' => Some(Token::Op(Op::Ampersand)),
                '(' => Some(Token::Punct(Punct::LParen)),
                ')' => Some(Token::Punct(Punct::RParen)),
                '{' => Some(Token::Punct(Punct::LBracket)),
                '}' => Some(Token::Punct(Punct::RBracket)),
                _ => self.read_phrase(c),
            },
            None => None,
        }
    }

    pub fn get_tokens(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while let Some(t) = self.next_token() {
            tokens.push(t);
        }
        tokens
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        self.next_token()
    }
}

fn is_name(c: &char) -> bool {
    c.is_alphabetic() || *c == '-' || *c == '_' || *c == '/'
}

#[cfg(test)]
mod lexer_tests {
    use super::{Op, Token, Lexer};

    #[test]
    fn test_tokenizer() {
        let mut lexer = Lexer::new("ls | grep cargo");
        let expected = [
            Token::Word("ls".to_string()),
            Token::Op(Op::Pipe),
            Token::Word("grep".to_string()),
            Token::Word("cargo".to_string()),
        ];
        for token in &expected {
            assert_eq!(*token, lexer.next().unwrap())
        }
    }
}
