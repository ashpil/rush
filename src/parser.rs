use crate::lexer::Token::*;
use crate::lexer::{Lexer, Op};
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Simple(Vec<String>),
    Function(Function),
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Box<Cmd>, Box<Cmd>),
    Or(Box<Cmd>, Box<Cmd>),
    Not(Box<Cmd>),
}

#[derive(Debug, PartialEq)]
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

    pub fn get(&mut self) -> Result<Cmd, String> {
        self.get_and()
    }

    pub fn get_and(&mut self) -> Result<Cmd, String> {
        let mut node = self.get_pipe()?;
        while let Some(Op(Op::And)) | Some(Op(Op::Or)) = self.lexer.peek() {
            if let Some(Op(Op::And)) = self.lexer.next() {
                node = Cmd::And(Box::new(node), Box::new(self.get_pipe()?));
            } else {
                node = Cmd::Or(Box::new(node), Box::new(self.get_pipe()?));
            }
        }
        Ok(node)
    }

    pub fn get_pipe(&mut self) -> Result<Cmd, String> {
        let mut node = self.get_simple()?;
        while let Some(Op(Op::Pipe)) = self.lexer.peek() {
            self.lexer.next();
            node = Cmd::Pipeline(Box::new(node), Box::new(self.get_simple()?));
        }
        Ok(node)
    }

    pub fn get_simple(&mut self) -> Result<Cmd, &str> {
        if let Some(Op(Op::Bang)) = self.lexer.peek() {
            self.lexer.next();
            Ok(Cmd::Not(Box::new(self.get_simple()?)))
        } else {
            let mut result = Vec::new();
            while let Some(Word(_)) = self.lexer.peek() {
                if let Some(Word(word)) = self.lexer.next() {
                    result.push(word);
                }
            }
            if result.len() == 0 {
                Err("Rush error: expected command but found none")
            } else {
                Ok(Cmd::Simple(result))
            }
        }
    }
}

#[cfg(test)]
mod parser_tests {
    use super::{Cmd::*, Parser};
    use crate::lexer::Lexer;

    #[test]
    fn test_and() {
        let lexer = Lexer::new("ls | grep cargo && pwd");
        let mut parser = Parser::new(lexer);
        let expected = And(
            Box::new(Pipeline(
                Box::new(Simple(vec!("ls".to_string()))),
                Box::new(Simple(vec!("grep".to_string(), "cargo".to_string()))),
            )),
            Box::new(Simple(vec!("pwd".to_string())))
        );
        assert_eq!(expected, parser.get())
    }

    #[test]
    fn test_pipes() {
        let lexer = Lexer::new("ls | grep cargo");
        let mut parser = Parser::new(lexer);
        let expected = Pipeline(
            Box::new(Simple(vec!("ls".to_string()))),
            Box::new(Simple(vec!("grep".to_string(), "cargo".to_string()))),
        );
        assert_eq!(expected, parser.get())
    }

    #[test]
    fn test_simple() {
        let lexer = Lexer::new("ls -ltr");
        let mut parser = Parser::new(lexer);
        let expected = Simple(vec!("ls".to_string(), "-ltr".to_string()));
        assert_eq!(expected, parser.get())
    }
}
