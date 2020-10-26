use nix::unistd::Uid;
use os_pipe::{dup_stderr, dup_stdin, dup_stdout, PipeReader, PipeWriter};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Stdio, self};

// My own, less nasty version of BufRead::lines().
// Returns an Option rather Option<Result>,
// and keeps the newline
#[derive(Debug)]
pub struct Lines<B> {
    buf: B,
}

impl<B> Lines<B> {
    pub fn new(buf: B) -> Lines<B> {
        Lines { buf }
    }
}

impl<B: BufRead> Iterator for Lines<B> {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut buf = String::new();
        match self.buf.read_line(&mut buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(buf),
        }
    }
}

pub struct Shell {
    lines: Lines<Box<dyn BufRead>>,
    interactive: bool,
    positional: Vec<String>,
    name: String,
    pub vars: HashMap<String, String>,
}

impl Shell {
    pub fn new(file: Option<String>) -> Shell {
        let (lines, interactive, name): (Lines<Box<dyn BufRead>>, bool, String) =
            if let Some(filename) = file {
                (
                    Lines::new(Box::new(BufReader::new(fs::File::open(&filename).unwrap()))),
                    false,
                    filename,
                )
            } else {
                (
                    Lines::new(Box::new(BufReader::new(io::stdin()))),
                    true,
                    String::from("rush"),
                )
            };
        Shell {
            lines,
            interactive,
            positional: Vec::new(),
            name,
            vars: HashMap::new(),
        }
    }

    pub fn get_pos(&self, n: u32) -> Option<&String> {
        self.positional.get((n - 1) as usize)
    }

    pub fn set_pos(&mut self, pos: Vec<String>) {
        self.positional = pos;
    }

    pub fn is_interactive(&self) -> bool {
        self.interactive
    }

    pub fn next_prompt(&mut self, prompt: &str) -> Option<String> {
        if self.is_interactive() {
            print!("{}", prompt);
            io::stdout().flush().unwrap();
        }
        self.lines.next()
    }

    // Not super satisfied with this as it is returning a String when it could be a 
    // reference, but this also allows handling stuff like $@ right here, as that would need to be 
    // stitched together here and thus it would own the value.
    // Also, env:: calls in Rust seem to return ownership rather than references, which is
    // nasty.
    pub fn get_var(&self, key: &str) -> Option<String> {
        if let Ok(num) = key.parse::<u32>() {
            if num == 0 {
                Some(self.name.clone())
            } else {
                self.get_pos(num).map(String::from)
            }
        } else {
            match key {
                "@" | "*" => Some(self.positional.join(" ")), // these are technically more complicated but it works for now
                "#" => Some(self.positional.len().to_string()), 
                "$" => Some(process::id().to_string()), 
                _ => self
                    .vars
                    .get(key)
                    .map_or(env::var(key).ok(), |s| Some(String::from(s))),
            }
        }
    }

    pub fn set_var(&mut self, key: String, val: String) {
        if env::var_os(&key).is_some() {
            env::set_var(key, val);
        } else {
            self.vars.insert(key, val);
        }
    }
}

impl Iterator for Shell {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        if self.is_interactive() {
            if Uid::current().is_root() {
                print!("#> ");
            } else {
                print!("$> ");
            }
            io::stdout().flush().unwrap();
        }
        self.lines.next()
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
