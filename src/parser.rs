use crate::lexer::Token::*;
use crate::lexer::{Lexer, Op};
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

#[derive(Debug)]
pub enum Cmd {
    Simple(Vec<String>),
    Function(Function),
    Pipeline(Vec<Cmd>),
    None,
}

#[derive(Debug)]
pub struct Function {
    name: String,
    body: Box<Cmd>,
}

impl Parser<'_> {
    pub fn new(lexer: Lexer) -> Parser {
        Parser {
            lexer: lexer.peekable(),
        }
    }

    pub fn get(&mut self) -> Cmd {
        let mut result = self.get_simple(None).unwrap();
        while let Some(token) = self.lexer.next() {
            match token {
                Op(Op::Pipe) => result = self.get_pipe(result),
                Word(word) => result = self.get_simple(Some(word)).unwrap(), 
                _ => (),
            }
        }
        result
    }

    pub fn get_pipe(&mut self, cmd: Cmd) -> Cmd {
        let mut result = vec!(cmd);
        while let Some(simple) = self.get_simple(None) {
            result.push(simple);
            self.lexer.next();
        }
        if result.len() != 1 {
            Cmd::Pipeline(result)
        } else {
            result.remove(0)
        }
    }

    pub fn get_simple(&mut self, word: Option<String>) -> Option<Cmd> {
        let mut result = Vec::new();
        if let Some(start) = word {
            result.push(start);
        }
        while let Some(Word(_)) = self.lexer.peek() {
            if let Some(Word(word)) = self.lexer.next() {
                result.push(word);
            }
        }
        if result.len() != 0 {
            Some(Cmd::Simple(result))
        } else {
            None
        }
    }
}
