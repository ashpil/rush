use std::io::{self, BufRead, BufReader, Write};
use std::fs;

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

pub enum Mode {
    Interactive(io::Stdout, Lines<Box<dyn BufRead>>),
    File(Lines<Box<dyn BufRead>>),
}

impl Mode {
    pub fn new(file: Option<String>) -> Mode {
        match file {
            Some(filename) => {
                Mode::File(Lines::new(Box::new(BufReader::new(fs::File::open(filename).unwrap()))))
            },
            None => {
                Mode::Interactive(io::stdout(), Lines::new(Box::new(BufReader::new(io::stdin()))))
            },
        }
    }
    pub fn next_prompt(&mut self, prompt: &str) -> Option<String> {
        let (stdout, lines) = match self {
            Mode::Interactive(stdout, lines) => (Some(stdout), lines),
            Mode::File(lines) => (None, lines),
        };
        if let Some(stdout) = stdout {
            print!("{}", prompt);
            stdout.flush().unwrap();
        }
        lines.next()
    }
}

impl Iterator for Mode {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let (stdout, lines) = match self {
            Mode::Interactive(stdout, lines) => (Some(stdout), lines),
            Mode::File(lines) => (None, lines),
        };
        if let Some(stdout) = stdout {
            print!("~> ");
            stdout.flush().unwrap();
        }
        lines.next()
    }

}
