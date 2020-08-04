use crate::helpers::Mode;
use crate::lexer::Token::*;
use crate::lexer::{Lexer, Op};
use os_pipe::{dup_stderr, dup_stdin, dup_stdout, pipe, PipeReader, PipeWriter};
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::iter::Peekable;
use std::process::Stdio;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Simple(Simple),
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Box<Cmd>, Box<Cmd>),
    Or(Box<Cmd>, Box<Cmd>),
    Not(Box<Cmd>),
}

// Keeps track of io in one spot before it's put into a command
pub struct Io {
    stdin: Rc<RefCell<Fd>>,
    stdout: Rc<RefCell<Fd>>,
    stderr: Rc<RefCell<Fd>>,
}

impl Io {
    fn new() -> Io {
        Io {
            stdin: Rc::new(RefCell::new(Fd::Stdin)),
            stdout: Rc::new(RefCell::new(Fd::Stdout)),
            stderr: Rc::new(RefCell::new(Fd::Stderr)),
        }
    }

    fn set_stdin(&mut self, fd: Rc<RefCell<Fd>>) {
        self.stdin = fd;
    }

    fn set_stdout(&mut self, fd: Rc<RefCell<Fd>>) {
        self.stdout = fd;
    }

    fn set_stderr(&mut self, fd: Rc<RefCell<Fd>>) {
        self.stderr = fd;
    }
}

// The most basic command - it, its arguments, and its redirections.
#[derive(Debug, PartialEq)]
pub struct Simple {
    pub cmd: String,
    pub args: Vec<String>,
    pub stdin: Rc<RefCell<Fd>>,
    pub stdout: Rc<RefCell<Fd>>,
    pub stderr: Rc<RefCell<Fd>>,
}

impl Simple {
    fn new(cmd: String, args: Vec<String>, io: Io) -> Simple {
        Simple {
            cmd,
            args,
            stdin: io.stdin,
            stdout: io.stdout,
            stderr: io.stderr,
        }
    }
}

// File descriptor - somewhat a misnomer now but it's nice and short.
// Keeps track of the various ports a stdio could be connected to.
#[derive(Debug)]
pub enum Fd {
    Stdin,
    Stdout,
    Stderr,
    Inherit,
    PipeOut(PipeWriter),
    PipeIn(PipeReader),
    FileName(String),
    FileNameAppend(String),
    RawFile(File),
}

impl PartialEq for Fd {
    fn eq(&self, other: &Self) -> bool {
        self.variant() == other.variant()
    }
}

impl Fd {
    fn variant(&self) -> &str {
        match *self {
            Fd::Stdin => "Stdin",
            Fd::Stdout => "Stdout",
            Fd::Stderr => "Stderr",
            Fd::Inherit => "Inherit",
            Fd::PipeOut(_) => "PipeOut",
            Fd::PipeIn(_) => "PipeIn",
            Fd::FileName(_) => "FileName",
            Fd::FileNameAppend(_) => "FileNameAppend",
            Fd::RawFile(_) => "RawFile", // Not completely accurate, but I think fine for now
        }
    }

    // Gets an stdin - all same here as stdout, except that a file is opened, not created
    pub fn get_stdin(&mut self) -> Option<Stdio> {
        match self {
            Fd::FileName(name) => match File::open(&name) {
                Ok(file) => {
                    *self = Fd::RawFile(file.try_clone().unwrap());
                    Some(Stdio::from(file))
                }
                Err(e) => {
                    eprintln!("rush: {}: {}", name, e);
                    None
                }
            },
            _ => self.get_stdout(),
        }
    }

    // All the ways a Fd could be converted to a Stdio
    // What's the proper way to deal with all of these dup unwraps?
    // What is their fail condition?
    pub fn get_stdout(&mut self) -> Option<Stdio> {
        match self {
            Fd::Stdin => Some(Stdio::from(dup_stdin().unwrap())),
            Fd::Stdout => Some(Stdio::from(dup_stdout().unwrap())),
            Fd::Stderr => Some(Stdio::from(dup_stderr().unwrap())),
            Fd::Inherit => Some(Stdio::inherit()),
            Fd::PipeOut(writer) => Some(Stdio::from(writer.try_clone().unwrap())),
            Fd::PipeIn(reader) => Some(Stdio::from(reader.try_clone().unwrap())),
            Fd::RawFile(file) => Some(Stdio::from(file.try_clone().unwrap())),
            Fd::FileName(name) => match File::create(&name) {
                Ok(file) => {
                    *self = Fd::RawFile(file.try_clone().unwrap());
                    Some(Stdio::from(file))
                }
                Err(e) => {
                    eprintln!("rush: {}: {}", name, e);
                    None
                }
            },
            Fd::FileNameAppend(name) => {
                match OpenOptions::new().append(true).create(true).open(&name) {
                    Ok(file) => {
                        *self = Fd::RawFile(file.try_clone().unwrap());
                        Some(Stdio::from(file))
                    }
                    Err(e) => {
                        eprintln!("rush: {}: {}", name, e);
                        None
                    }
                }
            }
        }
    }

    pub fn get_stderr(&mut self) -> Option<Stdio> {
        self.get_stdout()
    }
}

// The parser struct. Keeps track of current location in a peekable iter of tokens
pub struct Parser {
    mode: Rc<RefCell<Mode>>,
    lexer: Peekable<Lexer>,
}

impl Parser {
    pub fn new(lexer: Lexer, mode: Rc<RefCell<Mode>>) -> Parser {
        Parser {
            mode,
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

            let mut io = Io::new();

            loop {
                match self.lexer.peek() {
                    Some(Word(_)) => {
                        if let Some(Word(word)) = self.lexer.next() {
                            result.push(word);
                        }
                    }
                    Some(Op(Op::Less)) => {
                        self.lexer.next();
                        io.set_stdin(self.token_to_fd(&io)?);
                    },
                    Some(Op(Op::More)) => {
                        self.lexer.next();
                        io.set_stdout(self.token_to_fd(&io)?);
                    },
                    Some(Integer(_)) => {
                        if let Some(Integer(int)) = self.lexer.next() {
                            self.lexer.next();
                            match int {
                                0 => io.set_stdin(self.token_to_fd(&io)?),
                                1 => io.set_stdout(self.token_to_fd(&io)?),
                                2 => io.set_stderr(self.token_to_fd(&io)?),
                                _ => todo!(),
                            }
                        }
                    },
                    _ => break,
                }
            }
            if result.is_empty() {
                Err(String::from("rush: expected command but found none"))
            } else {
                Ok(Cmd::Simple(Simple::new(result.remove(0), result, io)))
            }
        }
    }

    fn token_to_fd(&mut self, io: &Io) -> Result<Rc<RefCell<Fd>>, String> {
        let error = String::from("rush: expected redirection location but found none");
        if let Some(token) = self.lexer.next() {
            match token {
                Op(Op::Ampersand) => {
                    if let Some(Integer(i)) = self.lexer.next() {
                        Ok(Rc::clone(match i {
                            0 => &io.stdin,
                            1 => &io.stdout,
                            2 => &io.stderr,
                            _ => todo!(),
                        }))
                    } else {
                        Err(error)
                    }
                }
                Op(Op::More) => {
                    if let Some(Word(s)) = self.lexer.next() {
                        Ok(Rc::new(RefCell::new(Fd::FileNameAppend(s))))
                    } else {
                        Err(error)
                    }
                }
                Op(Op::Less) => {
                    if let Some(Word(mut s)) = self.lexer.next() {
                        s = format!("{}\n", s);
                        let (reader, mut writer) = pipe().unwrap();

                        while let Some(input) = self.mode.borrow_mut().next_prompt("> ") {
                            if input == s {
                                break;
                            } else {
                                writer.write_all(input.as_bytes()).unwrap();
                            }
                        }
                        Ok(Rc::new(RefCell::new(Fd::PipeIn(reader))))
                    } else {
                        Err(error)
                    }
                }
                Word(s) => Ok(Rc::new(RefCell::new(Fd::FileName(s)))),
                Integer(i) => Ok(Rc::new(RefCell::new(Fd::FileName(i.to_string())))),
                _ => Err(error),
            }
        } else {
            Err(error)
        }
    }
}

// TODO: Tests for redirection
#[cfg(test)]
mod parser_tests {
    use super::{Cmd, Io, Parser, Simple};
    use crate::helpers::Mode;
    use crate::lexer::Lexer;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_and() {
        let mode = Rc::new(RefCell::new(Mode::new(None)));
        let lexer = Lexer::new("ls | grep cargo && pwd", Rc::clone(&mode));
        let mut parser = Parser::new(lexer, Rc::clone(&mode));
        let expected = Cmd::And(
            Box::new(Cmd::Pipeline(
                Box::new(Cmd::Simple(Simple::new(
                    String::from("ls"),
                    vec![],
                    Io::new(),
                ))),
                Box::new(Cmd::Simple(Simple::new(
                    String::from("grep"),
                    vec![String::from("cargo")],
                    Io::new(),
                ))),
            )),
            Box::new(Cmd::Simple(Simple::new(
                String::from("pwd"),
                vec![],
                Io::new(),
            ))),
        );
        assert_eq!(expected, parser.get().unwrap())
    }

    #[test]
    fn test_pipes() {
        let mode = Rc::new(RefCell::new(Mode::new(None)));
        let lexer = Lexer::new("ls | grep cargo", Rc::clone(&mode));
        let mut parser = Parser::new(lexer, Rc::clone(&mode));
        let expected = Cmd::Pipeline(
            Box::new(Cmd::Simple(Simple::new(
                String::from("ls"),
                vec![],
                Io::new(),
            ))),
            Box::new(Cmd::Simple(Simple::new(
                String::from("grep"),
                vec![String::from("cargo")],
                Io::new(),
            ))),
        );
        assert_eq!(expected, parser.get().unwrap())
    }

    #[test]
    fn test_simple() {
        let mode = Rc::new(RefCell::new(Mode::new(None)));
        let lexer = Lexer::new("ls -ltr", Rc::clone(&mode));
        let mut parser = Parser::new(lexer, Rc::clone(&mode));
        let expected = Cmd::Simple(Simple::new(
            String::from("ls"),
            vec![String::from("-ltr")],
            Io::new(),
        ));
        assert_eq!(expected, parser.get().unwrap())
    }
}
