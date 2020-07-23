use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq)]
pub enum Token {
    Word(String),
    Integer(u32),
    Operator(Operator),
    Punct(Punct),
}

#[derive(Debug, PartialEq)]
pub enum Operator {
    Plus,
    Minus,
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

    fn peek(&mut self) -> Option<&char> {
        self.text.peek()
    }

    fn next(&mut self) -> Option<char> {
        self.text.next()
    }

    fn skip_whitespace(&mut self) {
        loop {
            let next = self.peek();
            if !(next.is_some() && next.unwrap().is_whitespace()) {
                break;
            }
            self.next();
        }
    }

    fn read_phrase(&mut self, c: char) -> Option<Token> {
        let mut phrase = c.to_string();
        if is_name(&c) {
            loop {
                let next = self.peek();
                if !(next.is_some() && is_name(next.unwrap())) {
                    break;
                }
                phrase.push(self.next().unwrap());
            }
            Some(Token::Word(phrase))
        } else if c.is_digit(10) {
            loop {
                let next = self.peek();
                if !(next.is_some() && next.unwrap().is_digit(10)) {
                    break;
                }
                phrase.push(self.next().unwrap());
            }
            Some(Token::Integer(phrase.parse::<u32>().unwrap()))
        } else {
            None
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        match self.next() {
            Some(c) => match c {
                '+' => Some(Token::Operator(Operator::Plus)),
                '(' => Some(Token::Punct(Punct::LParen)),
                ')' => Some(Token::Punct(Punct::RParen)),
                '{' => Some(Token::Punct(Punct::LBracket)),
                '}' => Some(Token::Punct(Punct::RBracket)),
                '-' => {
                    let next = self.peek();
                    if next.is_some() && !next.unwrap().is_whitespace() {
                        self.read_phrase(c)
                    } else {
                        Some(Token::Operator(Operator::Minus))
                    }
                }
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
    use super::{Operator, Token, Tokenizer};

    #[test]
    fn test_tokenizer() {
        let mut lexer = Lexer::new("145 + word");
        let expected = [
            Token::Integer(145),
            Token::Operator(Operator::Plus),
            Token::Word("word".to_string()),
        ];
        for token in &expected {
            assert_eq!(*token, lexer.next_token().unwrap())
        }
    }
}
