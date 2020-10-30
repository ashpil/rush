use crate::builtins;
use crate::helpers::{Fd, Shell};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::{Cmd, Simple};
use os_pipe::{pipe, PipeReader, PipeWriter};
use std::process::Command;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::Read;

// This is useful to keep track of what each command does with its STDs
#[derive(Debug)]
struct CmdMeta {
    stdin: Option<PipeReader>,
    stdout: Option<PipeWriter>,
}

impl CmdMeta {
    fn inherit() -> CmdMeta {
        CmdMeta {
            stdin: None,
            stdout: None,
        }
    }

    fn pipe_out(writer: PipeWriter) -> CmdMeta {
        CmdMeta {
            stdin: None,
            stdout: Some(writer),
        }
    }

    fn new_in(self, reader: PipeReader) -> CmdMeta {
        CmdMeta {
            stdin: Some(reader),
            stdout: self.stdout,
        }
    }
}

pub struct Runner {
    shell: Rc<RefCell<Shell>>,
}

impl Runner {
    pub fn new(shell: Rc<RefCell<Shell>>) -> Runner {
        Runner {
            shell,
        }
    }

    pub fn execute(&self, ast: Cmd, capture: bool) -> Option<String> {
        if capture {
            let (mut reader, writer) = pipe().unwrap();
            self.visit(ast, CmdMeta::pipe_out(writer));
            let mut output = String::new();
            reader.read_to_string(&mut output).unwrap();
            Some(output)
        } else {
            self.visit(ast, CmdMeta::inherit());
            None
        }
    }

    // Probably not ideal for all of these to return a bool,
    // but it works for now. Once I figure out what's non-ideal
    // about it, I'll fix it
    fn visit(&self, node: Cmd, stdio: CmdMeta) -> bool {
        self.expand_alias_then_visit(node, stdio, None)
    }

    fn expand_alias_then_visit(
        &self,
        node: Cmd,
        stdio: CmdMeta,
        prev_alias: Option<String>,
    ) -> bool {
        match node {
            Cmd::Simple(simple) => {
                if (prev_alias.as_ref() != Some(&simple.cmd))
                    && self.shell.borrow().aliases.contains_key(&simple.cmd)
                {
                    let aliased_cmd = simple.cmd.clone();
                    let expanded = self.expand_alias(simple);
                    self.expand_alias_then_visit(expanded, stdio, Some(aliased_cmd))
                } else {
                    self.visit_simple(simple, stdio)
                }
            }
            Cmd::Pipeline(cmd0, cmd1) => self.visit_pipe(*cmd0, *cmd1, stdio),
            Cmd::And(cmd0, cmd1) => self.visit_and(*cmd0, *cmd1, stdio),
            Cmd::Or(cmd0, cmd1) => self.visit_or(*cmd0, *cmd1, stdio),
            Cmd::Not(cmd) => self.visit_not(*cmd, stdio),
            Cmd::Empty => true,
        }
    }

    fn expand_alias(&self, cmd: Simple) -> Cmd {
        let substitution = &self.shell.borrow().aliases[&cmd.cmd];
        let lexer = Lexer::new(substitution, Rc::clone(&self.shell));
        let mut parser = Parser::new(lexer, Rc::clone(&self.shell));

        if let Ok(expanded) = parser.get() {
            fn move_args(expanding: Cmd, parent: Simple) -> Cmd {
                match expanding {
                    Cmd::Simple(mut new_simple) => {
                        new_simple.args.extend(parent.args);
                        Cmd::Simple(new_simple)
                    }
                    Cmd::Pipeline(lhs, rhs) => {
                        Cmd::Pipeline(lhs, Box::new(move_args(*rhs, parent)))
                    }
                    Cmd::And(lhs, rhs) => Cmd::And(lhs, Box::new(move_args(*rhs, parent))),
                    Cmd::Or(lhs, rhs) => Cmd::Or(lhs, Box::new(move_args(*rhs, parent))),
                    Cmd::Not(not) => Cmd::Not(Box::new(move_args(*not, parent))),
                    Cmd::Empty => Cmd::Empty,
                }
            }

            fn propagate_env(expanding: Cmd, parent: &Simple) -> Cmd {
                match expanding {
                    Cmd::Simple(mut new_simple) => {
                        new_simple.env = parent.env.clone();
                        Cmd::Simple(new_simple)
                    }
                    Cmd::Pipeline(lhs, rhs) => Cmd::Pipeline(
                        Box::new(propagate_env(*lhs, parent)),
                        Box::new(propagate_env(*rhs, parent)),
                    ),
                    Cmd::And(lhs, rhs) => Cmd::And(
                        Box::new(propagate_env(*lhs, parent)),
                        Box::new(propagate_env(*rhs, parent)),
                    ),
                    Cmd::Or(lhs, rhs) => Cmd::Or(
                        Box::new(propagate_env(*lhs, parent)),
                        Box::new(propagate_env(*rhs, parent)),
                    ),
                    Cmd::Not(not) => Cmd::Not(Box::new(propagate_env(*not, parent))),
                    Cmd::Empty => Cmd::Empty,
                }
            }

            move_args(propagate_env(expanded, &cmd), cmd)
        } else {
            let mut cmd = cmd;
            cmd.cmd = if cmd.args.is_empty() {
                "".to_string()
            } else {
                cmd.args.remove(0)
            };
            Cmd::Simple(cmd)
        }
    }

    fn visit_not(&self, cmd: Cmd, stdio: CmdMeta) -> bool {
        let result = self.visit(cmd, stdio);
        !result
    }

    fn visit_or(&self, left: Cmd, right: Cmd, stdio: CmdMeta) -> bool {
        let left = self.visit(left, CmdMeta::inherit());
        if left {
            left
        } else {
            self.visit(right, stdio)
        }
    }

    fn visit_and(&self, left: Cmd, right: Cmd, stdio: CmdMeta) -> bool {
        let left = self.visit(left, CmdMeta::inherit());
        if left {
            self.visit(right, stdio)
        } else {
            left
        }
    }

    // We create a pipe, pass the writing end to the left, and modify the stdio
    // to have its stdin be the reading end.
    fn visit_pipe(&self, left: Cmd, right: Cmd, stdio: CmdMeta) -> bool {
        let (reader, writer) = pipe().unwrap();
        self.visit(left, CmdMeta::pipe_out(writer));
        self.visit(right, stdio.new_in(reader))
    }

    fn visit_simple(&self, mut simple: Simple, stdio: CmdMeta) -> bool {
        self.reconcile_io(&mut simple, stdio);
        match &simple.cmd[..] {
            "alias" => builtins::alias(&mut self.shell.borrow_mut().aliases, simple.args),
            "exit" => builtins::exit(simple.args),
            "cd" => builtins::cd(simple.args),
            "set" => builtins::set(simple.args, &self.shell),
            "unalias" => builtins::unalias(&mut self.shell.borrow_mut().aliases, simple.args),

            command => {
                let mut cmd = Command::new(command);
                cmd.args(&simple.args);

                if let Some(stdin) = simple.stdin.borrow_mut().get_stdin() {
                    cmd.stdin(stdin);
                } else {
                    return false;
                }
                if let Some(stdout) = simple.stdout.borrow_mut().get_stdout() {
                    cmd.stdout(stdout);
                } else {
                    return false;
                }
                if let Some(stderr) = simple.stdin.borrow_mut().get_stderr() {
                    cmd.stderr(stderr);
                } else {
                    return false;
                }
                if let Some(env) = simple.env {
                    cmd.envs(env);
                }

                match cmd.status() {
                    Ok(child) => child.success(),
                    Err(e) => {
                        eprintln!("rush: {}: {}", simple.cmd, e);
                        false
                    }
                }
            }
        }
    }

    // Takes the stdio and if stdio has priority, replaces stdout/stdin with it.
    fn reconcile_io(&self, simple: &mut Simple, stdio: CmdMeta) {
        if let Some(stdout) = stdio.stdout {
            if *simple.stdout.borrow() == Fd::Stdout {
                *simple.stdout.borrow_mut() = Fd::PipeOut(stdout);
            }
        }
        if let Some(stdin) = stdio.stdin {
            if *simple.stdin.borrow() == Fd::Stdin {
                *simple.stdin.borrow_mut() = Fd::PipeIn(stdin);
            }
        }
    }
}
// How do I test this module?
