// The Lexer does the work required before the AST can be built
// in the parser

use crate::helpers::Shell;
use std::cell::RefCell;
use std::iter::Peekable;
use std::rc::Rc;
use std::vec::IntoIter;

use self::Expand::*;

// Parent enum for tokens
#[derive(Debug, PartialEq)]
pub enum Token {
    Word(Vec<Expand>),
    Integer(u32),
    Assign(String, Vec<Expand>),
    Op(Op),
    Punct(Punct),
}

#[derive(Debug, PartialEq)]
pub enum Expand {
    Literal(String),
    Var(String),
    Tilde(Vec<Expand>),
    Brace(String, Action, Vec<Expand>),
}

// What the brace does expansion does:
// https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_06_02
// If true test for unset or null, if false, only unset
// For prefix/suffix, true for largest false for smallest
#[derive(Debug, PartialEq)]
pub enum Action {
    UseDefault(bool),
    AssignDefault(bool),
    IndicateError(bool),
    UseAlternate(bool),
    RmSmallestSuffix,
    RmLargestSuffix,
    RmSmallestPrefix,
    RmLargestPrefix,
    StringLength,
}

impl Expand {
    pub fn get_name(self) -> String {
        match self {
            Literal(s) | Var(s) | Brace(s, _, _) => s,
            Tilde(_) => panic!("you shouldn't be doing this"),
        }
    }
}

// Operators
#[derive(Debug, PartialEq)]
pub enum Op {
    Pipe,
    Ampersand,
    Bang,
    Or,
    And,
    Less,
    More,
}

// Punctuation
#[derive(Debug, PartialEq)]
pub enum Punct {
    LParen,
    RParen,
    Semicolon,
}

// This representation makes it's functions very nice and easy,
// but I'm not convinced that this is the most efficient/clean
// the struct itself can be

fn invalid_var(c: char) -> bool {
    matches!(
        c,
        '&' | '!' | '|' | '<' | '>' | '"' | '=' | ':' | '}' | '+' | '-' | '?' | '$' | '\\'
    ) || c.is_whitespace()
}

fn is_token_split(c: char) -> bool {
    matches!(c, '&' | '!' | '|' | '<' | '>' | '=') || c.is_whitespace()
}

pub struct Lexer {
    shell: Rc<RefCell<Shell>>,
    line: Peekable<IntoIter<char>>,
}

impl Lexer {
    pub fn new(line: &str, shell: Rc<RefCell<Shell>>) -> Lexer {
        Lexer {
            shell,
            line: line.chars().collect::<Vec<_>>().into_iter().peekable(),
        }
    }

    fn advance_line(&mut self) -> Result<(), String> {
        if let Some(s) = self.shell.borrow_mut().next_prompt("> ") {
            self.line = s.chars().collect::<Vec<_>>().into_iter().peekable();
            Ok(())
        } else {
            Err(String::from("expected more input but found one"))
        }
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.line.peek()
    }

    fn next_char(&mut self) -> Option<char> {
        self.line.next()
    }

    fn skip_whitespace(&mut self) {
        let mut next = self.peek_char();
        while next.is_some() && next.unwrap().is_whitespace() {
            self.next_char();
            next = self.peek_char();
        }
    }

    fn read_until(
        &mut self,
        consume: bool,
        keep_going: bool,
        break_cond: Box<dyn Fn(char) -> bool>,
    ) -> Result<Vec<Expand>, String> {
        let mut expandables = Vec::new();
        let mut cur_word = String::new();

        let mut next = self.peek_char();
        loop {
            match next {
                Some('\\') => {
                    self.next_char();
                    match self.next_char() {
                        Some('\n') => self.advance_line()?,
                        Some(c) => cur_word.push(c),
                        None => (),
                    }
                }
                Some(c) if break_cond(*c) => {
                    // This just makes assignment easier
                    if *c == '=' {
                        cur_word.push(self.next_char().unwrap());
                        expandables.push(Literal(cur_word));
                        cur_word = String::new();
                    } else {
                        if consume {
                            self.next_char();
                        }
                        break;
                    }
                }
                Some('$') => {
                    if !cur_word.is_empty() {
                        expandables.push(Literal(cur_word));
                        cur_word = String::new();
                    }
                    self.next_char();
                    if let Some('{') = self.peek_char() {
                        fn get_action(null: bool, c: Option<char>) -> Option<Action> {
                            match c {
                                Some('-') => Some(Action::UseDefault(null)),
                                Some('=') => Some(Action::AssignDefault(null)),
                                Some('?') => Some(Action::IndicateError(null)),
                                Some('+') => Some(Action::UseAlternate(null)),
                                _ => None,
                            }
                        }

                        self.next_char();
                        let param = self.read_raw_until(invalid_var)?;

                        let action = match self.next_char() {
                            Some(':') => get_action(true, self.next_char()),
                            Some('%') => {
                                if let Some('%') = self.peek_char() {
                                    self.next_char();
                                    Some(Action::RmLargestSuffix)
                                } else {
                                    Some(Action::RmSmallestSuffix)
                                }
                            }
                            Some('#') => {
                                if let Some('#') = self.peek_char() {
                                    self.next_char();
                                    Some(Action::RmLargestPrefix)
                                } else {
                                    Some(Action::RmSmallestPrefix)
                                }
                            }
                            Some(' ') => return Err(String::from("bad substitution")),
                            c => get_action(false, c),
                        };

                        if let Some(a) = action {
                            let word = self.read_until(true, true, Box::new(|c| c == '}'))?;
                            expandables.push(Brace(param, a, word));
                        } else {
                            expandables.push(Var(param));
                        }
                    } else {
                        let name = self.read_raw_until(invalid_var)?;
                        expandables.push(Var(name));
                    }
                }
                Some('~') => {
                    if !cur_word.is_empty() {
                        expandables.push(Literal(cur_word));
                        cur_word = String::new();
                    }
                    self.next_char();

                    let tilde = self.read_until(false, false, Box::new(invalid_var))?;
                    expandables.push(Tilde(tilde));
                }
                Some('"') => {
                    if !cur_word.is_empty() {
                        expandables.push(Literal(cur_word));
                        cur_word = String::new();
                    }
                    self.next_char();

                    let mut result = self.read_until(true, true, Box::new(|c| c == '"'))?;
                    if result.is_empty() {
                        expandables.push(Literal(String::new()));
                    } else {
                        expandables.append(&mut result);
                    }
                }
                Some('\'') => {
                    self.next_char();
                    let mut phrase = String::new();
                    loop {
                        match self.next_char() {
                            Some('\'') => break,
                            Some(c) => phrase.push(c),
                            None => self.advance_line()?,
                        }
                    }
                    expandables.push(Literal(phrase));
                }
                Some(_) => cur_word.push(self.next_char().unwrap()),
                None => {
                    if keep_going {
                        self.advance_line()?;
                    } else {
                        break;
                    }
                }
            }
            next = self.peek_char();
        }
        if !cur_word.is_empty() {
            expandables.push(Literal(cur_word));
        }
        Ok(expandables)
    }

    // You can accomplish this same thing with just the function above,
    // but I think this is cleaner
    fn read_raw_until<F>(&mut self, break_cond: F) -> Result<String, String>
    where
        F: Fn(char) -> bool,
    {
        let mut word = String::new();
        while let Some(c) = self.peek_char() {
            match c {
                '\\' => {
                    self.next_char();
                    match self.next_char() {
                        Some('\n') => self.advance_line()?,
                        Some(c) => word.push(c),
                        None => (),
                    }
                }
                c if break_cond(*c) => break,
                _ => word.push(self.next_char().unwrap()),
            }
        }
        Ok(word)
    }

    // Of course, I still haven't added everything I'll need to yet
    fn next_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('|') => {
                self.next_char();
                if let Some('|') = self.peek_char() {
                    self.next_char();
                    Some(Token::Op(Op::Or))
                } else {
                    Some(Token::Op(Op::Pipe))
                }
            }
            Some('&') => {
                self.next_char();
                if let Some('&') = self.peek_char() {
                    self.next_char();
                    Some(Token::Op(Op::And))
                } else {
                    Some(Token::Op(Op::Ampersand))
                }
            }
            Some('>') => {
                self.next_char();
                Some(Token::Op(Op::More))
            }
            Some('<') => {
                self.next_char();
                Some(Token::Op(Op::Less))
            }
            Some('!') => {
                self.next_char();
                Some(Token::Op(Op::Bang))
            }
            Some('(') => {
                self.next_char();
                Some(Token::Punct(Punct::LParen))
            }
            Some(')') => {
                self.next_char();
                Some(Token::Punct(Punct::RParen))
            }
            Some(_) => match self.read_until(false, false, Box::new(is_token_split)) {
                Ok(w) => {
                    println!("The words I got: {:?}", w);
                    match &w[..] {
                        [Literal(s), ..]
                            if s.ends_with('=')
                                && s.chars().filter(|c| c.is_numeric()).count() != s.len() - 1 =>
                        {
                            let mut iter = w.into_iter();
                            let mut name = iter.next().unwrap().get_name();
                            name.pop();
                            Some(Token::Assign(name, iter.collect()))
                        }
                        [Literal(s)] => {
                            if let Ok(num) = s.parse::<u32>() {
                                Some(Token::Integer(num))
                            } else {
                                Some(Token::Word(w))
                            }
                        }
                        _ => Some(Token::Word(w)),
                    }
                }
                Err(e) => {
                    eprintln!("rush: {}", e);
                    None
                }
            },
            None => None,
        }
    }
}

impl Iterator for Lexer {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        self.next_token()
    }
}

// TODO: More tests
#[cfg(test)]
mod lexer_tests {
    use super::{Expand::*, Lexer, Op, Token::*};
    use crate::helpers::Shell;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn test_lexer() {
        let shell = Rc::new(RefCell::new(Shell::new(None)));
        let mut lexer = Lexer::new("exa -1 | grep cargo", Rc::clone(&shell));
        let expected = [
            Word(vec![Literal(String::from("exa"))]),
            Word(vec![Literal(String::from("-1"))]),
            Op(Op::Pipe),
            Word(vec![Literal(String::from("grep"))]),
            Word(vec![Literal(String::from("cargo"))]),
        ];
        for token in &expected {
            assert_eq!(*token, lexer.next().unwrap())
        }
    }
}
