use crate::lexer::Token::*;
use crate::lexer::{Lexer, Op};
use std::fs::File;
use std::iter::Peekable;
use std::os::unix::io::FromRawFd;
use std::process::Stdio;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Simple(Simple),
    Function(Function),
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Box<Cmd>, Box<Cmd>),
    Or(Box<Cmd>, Box<Cmd>),
    Not(Box<Cmd>),
}

#[derive(Debug)]
pub struct Simple {
    pub cmd: String,
    pub args: Vec<String>,
    pub stdin: Option<Stdio>,
    pub stdout: Option<Stdio>,
    pub stderr: Option<Stdio>,
}

#[derive(Debug)]
pub enum Fd {
    File(String),
    RawFd(i32),
}

impl PartialEq for Simple {
    fn eq(&self, other: &Self) -> bool {
        self.cmd == other.cmd && self.args == other.args
    }
}

impl Simple {
    fn new(cmd: String, args: Vec<String>) -> Simple {
        Simple {
            cmd,
            args,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    fn set_stdin(&mut self, fd: Fd) -> Result<(), String> {
        println!("Stdin set to: {:?}", fd);
        match fd {
            Fd::File(filename) => {
                if let Ok(file) = File::open(&filename) {
                    self.stdin = Some(Stdio::from(file));
                    Ok(())
                } else {
                    Err(format!("rush: {}: no such file or directory", filename))
                }
            }
            Fd::RawFd(i) => {
                self.stdin = Some(unsafe { Stdio::from_raw_fd(i) });
                Ok(())
            },
        }
    }

    fn set_stdout(&mut self, fd: Fd) -> Result<(), String> {
        println!("Stdout set to: {:?}", fd);
        match fd {
            Fd::File(filename) => {
                if let Ok(file) = File::open(&filename) {
                    self.stdout = Some(Stdio::from(file));
                    Ok(())
                } else {
                    self.stdout = Some(Stdio::from(File::create(&filename).unwrap()));
                    Ok(())
                }
            }
            Fd::RawFd(i) => {
                self.stdout = Some(unsafe { Stdio::from_raw_fd(i) });
                Ok(())
            },
        }
    }

    fn set_stderr(&mut self, fd: Fd) -> Result<(), String> {
        println!("Stderr set to: {:?}", fd);
        match fd {
            Fd::File(filename) => {
                if let Ok(file) = File::open(&filename) {
                    self.stderr = Some(Stdio::from(file));
                    Ok(())
                } else {
                    self.stderr = Some(Stdio::from(File::create(&filename).unwrap()));
                    Ok(())
                }
            }
            Fd::RawFd(i) => {
                self.stderr = Some(unsafe { Stdio::from_raw_fd(i) });
                Ok(())
            },
        }
    }
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

    pub fn get_simple(&mut self) -> Result<Cmd, String> {
        if let Some(Op(Op::Bang)) = self.lexer.peek() {
            self.lexer.next();
            Ok(Cmd::Not(Box::new(self.get_simple()?)))
        } else {
            let mut result = Vec::new();

            let (stdin, stdout, stderr) = self.update_stds(None, None, None)?;

            while let Some(Word(_)) = self.lexer.peek() {
                if let Some(Word(word)) = self.lexer.next() {
                    result.push(word);
                }
            }

            let (stdin, stdout, stderr) = self.update_stds(stdin, stdout, stderr)?;

            if result.len() == 0 {
                Err(String::from("rush: expected command but found none"))
            } else {
                let mut simple = Simple::new(result.remove(0), result);

                if let Some(fd) = stdin {
                    simple.set_stdin(fd)?;
                }
                if let Some(fd) = stdout {
                    simple.set_stdout(fd)?;
                }
                if let Some(fd) = stderr {
                    simple.set_stderr(fd)?;
                }

                Ok(Cmd::Simple(simple))
            }
        }
    }

    fn update_stds(
        &mut self,
        mut stdin: Option<Fd>,
        mut stdout: Option<Fd>,
        mut stderr: Option<Fd>,
    ) -> Result<(Option<Fd>, Option<Fd>, Option<Fd>), String> {
        loop {
            match self.lexer.peek() {
                Some(Op(Op::Less)) => {
                    self.lexer.next();
                    stdin = Some(self.token_to_fd()?);
                }
                Some(Op(Op::More)) => {
                    self.lexer.next();
                    stdout = Some(self.token_to_fd()?);
                }
                Some(Integer(_)) => {
                    if let Some(Integer(int)) = self.lexer.next() {
                        self.lexer.next();
                        match int {
                            0 => stdin = Some(self.token_to_fd()?),
                            1 => stdout = Some(self.token_to_fd()?),
                            2 => stderr = Some(self.token_to_fd()?),
                            _ => unimplemented!(),
                        }
                    }
                }
                _ => break Ok((stdin, stdout, stderr)),
            }
        }
    }

    fn token_to_fd(&mut self) -> Result<Fd, String> {
        let error = String::from("Rush error: expected redirection location but found none");
        if let Some(token) = self.lexer.next() {
            match token {
                Op(Op::Ampersand) => {
                    if let Some(Integer(i)) = self.lexer.next() {
                        Ok(Fd::RawFd(i as i32))
                    } else {
                        Err(error)
                    }
                }
                Word(s) => Ok(Fd::File(s)),
                Integer(i) => Ok(Fd::File(i.to_string())),
                _ => Err(error),
            }
        } else {
            Err(error)
        }
    }
}

#[cfg(test)]
mod parser_tests {
    use super::{Cmd, Parser, Simple};
    use crate::lexer::Lexer;

    #[test]
    fn test_and() {
        let lexer = Lexer::new("ls | grep cargo && pwd");
        let mut parser = Parser::new(lexer);
        let expected = Cmd::And(
            Box::new(Cmd::Pipeline(
                Box::new(Cmd::Simple(Simple::new(String::from("ls"), vec![]))),
                Box::new(Cmd::Simple(Simple::new(
                    String::from("grep"),
                    vec![String::from("cargo")],
                ))),
            )),
            Box::new(Cmd::Simple(Simple::new(String::from("pwd"), vec![]))),
        );
        assert_eq!(expected, parser.get().unwrap())
    }

    #[test]
    fn test_pipes() {
        let lexer = Lexer::new("ls | grep cargo");
        let mut parser = Parser::new(lexer);
        let expected = Cmd::Pipeline(
            Box::new(Cmd::Simple(Simple::new(String::from("ls"), vec![]))),
            Box::new(Cmd::Simple(Simple::new(
                String::from("grep"),
                vec![String::from("cargo")],
            ))),
        );
        assert_eq!(expected, parser.get().unwrap())
    }

    #[test]
    fn test_simple() {
        let lexer = Lexer::new("ls -ltr");
        let mut parser = Parser::new(lexer);
        let expected = Cmd::Simple(Simple::new(String::from("ls"), vec![String::from("-ltr")]));
        assert_eq!(expected, parser.get().unwrap())
    }
}
