use crate::helpers::{Fd, Shell};
use crate::lexer::Token::{self, *};
use crate::lexer::{
    Action,
    Expand::{self, *},
    Op,
};
use nix::unistd::User;
use os_pipe::pipe;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::iter::Peekable;
use std::process::exit;
use std::rc::Rc;
use crate::runner::Runner;

#[derive(Debug, PartialEq)]
pub enum Cmd {
    Simple(Simple),
    Pipeline(Box<Cmd>, Box<Cmd>),
    And(Box<Cmd>, Box<Cmd>),
    Or(Box<Cmd>, Box<Cmd>),
    Not(Box<Cmd>),
    Empty,
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
    pub env: Option<HashMap<String, String>>,
    pub stdin: Rc<RefCell<Fd>>,
    pub stdout: Rc<RefCell<Fd>>,
    pub stderr: Rc<RefCell<Fd>>,
}

impl Simple {
    fn new(cmd: String, args: Vec<String>, io: Io) -> Simple {
        Simple {
            cmd,
            args,
            env: None,
            stdin: io.stdin,
            stdout: io.stdout,
            stderr: io.stderr,
        }
    }

    fn add_env(&mut self, map: HashMap<String, String>) {
        self.env = Some(map);
    }
}

// The parser struct. Keeps track of current location in a peekable iter of tokens
pub struct Parser<I> 
where
    I: Iterator<Item = Token>
{
    shell: Rc<RefCell<Shell>>,
    lexer: Peekable<I>,
}

impl<I> Parser<I>
    where
        I: Iterator<Item = Token>
    {
    pub fn new(lexer: I, shell: Rc<RefCell<Shell>>) -> Parser<I> {
        Parser {
            shell,
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
            let mut map = HashMap::new();

            loop {
                match self.lexer.peek() {
                    Some(Word(_)) => {
                        if let Some(Word(mut expansions)) = self.lexer.next() {
                            if let [Literal(_)] = &expansions[..] {
                                result.push(expansions.pop().unwrap().get_name())
                            } else {
                                let word = self.expand_word(expansions);
                                if !word.is_empty() {
                                    result.push(word)
                                }
                            }
                        }
                    }
                    Some(Assign(_, _)) => {
                        if let Some(Assign(key, var)) = self.lexer.next() {
                            map.insert(key, self.expand_word(var));
                        }
                    }
                    Some(Op(Op::Less)) => {
                        self.lexer.next();
                        io.set_stdin(self.token_to_fd(&io)?);
                    }
                    Some(Op(Op::More)) => {
                        self.lexer.next();
                        io.set_stdout(self.token_to_fd(&io)?);
                    }
                    Some(Integer(_)) => {
                        if let Some(Integer(int)) = self.lexer.next() {
                            if let Some(Op(_)) = self.lexer.peek() {
                                self.lexer.next();
                                match int {
                                    0 => io.set_stdin(self.token_to_fd(&io)?),
                                    1 => io.set_stdout(self.token_to_fd(&io)?),
                                    2 => io.set_stderr(self.token_to_fd(&io)?),
                                    _ => todo!(),
                                }
                            } else {
                                result.push(int.to_string());
                            }
                        }
                    }
                    _ => break,
                }
            }
            if result.is_empty() {
                if map.is_empty() {
                    Err(String::from("rush: expected command but found none"))
                } else {
                    map = map
                        .into_iter()
                        .filter_map(|(k, v)| {
                            if env::var_os(&k).is_some() {
                                env::set_var(k, v);
                                None
                            } else {
                                Some((k, v))
                            }
                        })
                        .collect();
                    self.shell.borrow_mut().vars.extend(map);
                    Ok(Cmd::Empty)
                }
            } else {
                let mut cmd = Simple::new(result.remove(0), result, io);
                if !map.is_empty() {
                    cmd.add_env(map);
                }
                Ok(Cmd::Simple(cmd))
            }
        }
    }

    fn expand_word(&mut self, expansions: Vec<Expand>) -> String {
        let mut phrase = String::new();
        for word in expansions {
            match word {
                Literal(s) => phrase.push_str(&s),
                Tilde(word) => {
                    let s = self.expand_word(word);
                    if s.is_empty() || s.starts_with('/') {
                        phrase.push_str(&env::var("HOME").unwrap());
                        phrase.push_str(&s);
                    } else {
                        let mut strings = s.splitn(1, '/');
                        let name = strings.next().unwrap();
                        if let Some(user) = User::from_name(name).unwrap() {
                            phrase.push_str(user.dir.as_os_str().to_str().unwrap());
                            if let Some(path) = strings.next() {
                                phrase.push_str(path);
                            }
                        } else {
                            phrase.push('~');
                            phrase.push_str(name);
                        }
                    }
                }
                Var(s) => {
                    phrase.push_str(
                        &self.shell
                            .borrow()
                            .get_var(&s)
                            .unwrap_or_default()
                    );
                }
                Brace(key, action, word) => {
                    let val = self.shell.borrow().get_var(&key);
                    match action {
                        Action::UseDefault(null) => {
                            if let Some(s) = val {
                                if s == "" && null {
                                    phrase.push_str(&self.expand_word(word))
                                } else {
                                    phrase.push_str(&s)
                                }
                            } else {
                                phrase.push_str(&self.expand_word(word))
                            }
                        }
                        Action::AssignDefault(null) => {
                            if let Some(s) = val {
                                if s == "" && null {
                                    let expanded = self.expand_word(word);
                                    phrase.push_str(&expanded);
                                    self.shell.borrow_mut().set_var(key, expanded);
                                } else {
                                    phrase.push_str(&s)
                                }
                            } else {
                                let expanded = self.expand_word(word);
                                phrase.push_str(&expanded);
                                self.shell.borrow_mut().set_var(key, expanded);
                            }
                        }
                        Action::IndicateError(null) => {
                            if let Some(s) = val {
                                if s == "" && null {
                                    let message = self.expand_word(word);
                                    if message.is_empty() {
                                        eprintln!("rush: {}: parameter null", key);
                                    } else {
                                        eprintln!("rush: {}: {}", key, message);
                                    }
                                    if !self.shell.borrow().is_interactive() {
                                        exit(1);
                                    }
                                } else {
                                    phrase.push_str(&s)
                                }
                            } else {
                                let message = self.expand_word(word);
                                if message.is_empty() {
                                    eprintln!("rush: {}: parameter not set", key);
                                } else {
                                    eprintln!("rush: {}: {}", key, message);
                                }
                                if !self.shell.borrow().is_interactive() {
                                    exit(1);
                                }
                            }
                        }
                        Action::UseAlternate(null) => {
                            if let Some(s) = val {
                                if s != "" || !null {
                                    phrase.push_str(&self.expand_word(word))
                                }
                            }
                        }
                        Action::RmSmallestSuffix => todo!(),
                        Action::RmLargestSuffix => todo!(),
                        Action::RmSmallestPrefix => todo!(),
                        Action::RmLargestPrefix => todo!(),
                        Action::StringLength => todo!(),
                    }
                }
                Sub(e) => {
                    let mut parser = Parser::new(vec!(Word(e)).into_iter(), Rc::clone(&self.shell));

                    // This setup here allows me to do a surprisingly easy subshell.
                    // Though subshells typically seem to inherit everything I'm keeping in my
                    // `shell` variable at the moment?
                    if let Ok(command) = parser.get() {
                        #[cfg(debug_assertions)] // Only include when not built with `--release` flag
                        println!("\u{001b}[33m{:#?}\u{001b}[0m", command);

                        let mut output = Runner::new(Rc::clone(&parser.shell)).execute(command, true).unwrap();
                        output = output.replace(|c: char| c.is_whitespace(), " ");
                        phrase.push_str(output.trim());
                    }
                }
            }
        }
        phrase
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
                        Ok(Rc::new(RefCell::new(Fd::FileNameAppend(
                            self.expand_word(s),
                        ))))
                    } else {
                        Err(error)
                    }
                }
                Op(Op::Less) => {
                    if let Some(Word(s)) = self.lexer.next() {
                        let mut s = self.expand_word(s);
                        s = format!("{}\n", s);
                        let (reader, mut writer) = pipe().unwrap();

                        while let Some(input) = self.shell.borrow_mut().next_prompt("> ") {
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
                Word(s) => Ok(Rc::new(RefCell::new(Fd::FileName(self.expand_word(s))))),
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
    use crate::helpers::Shell;
    use crate::lexer::Lexer;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_and() {
        let shell = Rc::new(RefCell::new(Shell::new(None)));
        let lexer = Lexer::new("ls | grep cargo && pwd", Rc::clone(&shell));
        let mut parser = Parser::new(lexer, Rc::clone(&shell));
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
        let shell = Rc::new(RefCell::new(Shell::new(None)));
        let lexer = Lexer::new("ls | grep cargo", Rc::clone(&shell));
        let mut parser = Parser::new(lexer, Rc::clone(&shell));
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
        let shell = Rc::new(RefCell::new(Shell::new(None)));
        let lexer = Lexer::new("ls -ltr", Rc::clone(&shell));
        let mut parser = Parser::new(lexer, Rc::clone(&shell));
        let expected = Cmd::Simple(Simple::new(
            String::from("ls"),
            vec![String::from("-ltr")],
            Io::new(),
        ));
        assert_eq!(expected, parser.get().unwrap())
    }
}
