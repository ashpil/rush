use crate::lexer::{Lexer, Punct};
use crate::lexer::Token::*;
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
    ast: Option<AST>,
}

#[derive(Debug)]
pub enum AST {
    Basic(Vec<String>),
    Function(Function),
    None,
}

#[derive(Debug)]
pub struct Function {
    name: String,
    body: Box<AST>,
}

impl Parser<'_> {
    pub fn new(lexer: Lexer) -> Parser {
        Parser { lexer: lexer.peekable(), ast: None }
    }

    pub fn gen(&mut self) -> AST {
        let mut answer = AST::None;
        while let Some(token) = self.lexer.next() {
            match token {
                Word(word) => {
                    match self.lexer.peek() {
                        Some(Punct(Punct::LParen)) => {
                            self.lexer.next();
                            self.lexer.next();
                            self.lexer.next();
                            answer = AST::Function(Function { name: word, body: Box::new(self.gen()) });
                            self.lexer.next();
                        },
                        _ => {
                            let mut result = vec!(word);
                            while let Some(Word(word)) = self.lexer.next() {
                                println!("In word: {:?}", word);
                                result.push(word);
                            }
                            answer = AST::Basic(result);
                        },
                    }
                }
                _ => (),
            }
        }
        answer
    }
}
