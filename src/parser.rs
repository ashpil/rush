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
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Vec<Cmd>),
}

#[derive(Debug)]
pub struct Function {
    name: String,
    body: Box<Cmd>,
}

// This used to be much more complicated but I refractored it to be much simpler
impl Parser<'_> {
    pub fn new(lexer: Lexer) -> Parser {
        Parser {
            lexer: lexer.peekable(),
        }
    }

    pub fn get(&mut self) -> Cmd {
        self.get_pipe()
    }

    pub fn get_pipe(&mut self) -> Cmd {
        let mut node = self.get_simple();
        while let Some(Op(Op::Pipe)) = self.lexer.peek() {
            self.lexer.next();
            node = Cmd::Pipeline(Box::new(node), Box::new(self.get_simple()));
        }
        node
    }

    pub fn get_simple(&mut self) -> Cmd {
        let mut result = Vec::new();
        while let Some(Word(_)) = self.lexer.peek() {
            if let Some(Word(word)) = self.lexer.next() {
                result.push(word);
            }
        }
        Cmd::Simple(result)
    }
}
