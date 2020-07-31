use crate::parser::{Cmd, Fd, Simple};
use os_pipe::{pipe, PipeReader, PipeWriter};
use std::env;
use std::path::Path;
use std::process::{exit, Command};


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

pub fn execute(ast: Cmd) {
    visit(ast, CmdMeta::inherit());
}

fn visit(node: Cmd, stdio: CmdMeta) -> Option<bool> {
    match node {
        Cmd::Simple(simple) => visit_simple(simple, stdio),
        Cmd::Pipeline(cmd0, cmd1) => visit_pipe(*cmd0, *cmd1, stdio),
        Cmd::And(cmd0, cmd1) => visit_and(*cmd0, *cmd1, stdio),
        Cmd::Or(cmd0, cmd1) => visit_or(*cmd0, *cmd1, stdio),
        Cmd::Not(cmd) => visit_not(*cmd, stdio),
    }
}

fn visit_not(cmd: Cmd, stdio: CmdMeta) -> Option<bool> {
    let result = visit(cmd, stdio)?;
    Some(!result)
}

fn visit_or(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<bool> {
    let left = visit(left, CmdMeta::inherit())?;
    if !left {
        visit(right, stdio)
    } else {
        Some(left)
    }
}

fn visit_and(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<bool> {
    let left = visit(left, CmdMeta::inherit())?;
    if left {
        visit(right, stdio)
    } else {
        Some(left)
    }
}

// We create a pipe, pass the writing end to the left, and modify the stdio
// to have its stdin be the reading end.
fn visit_pipe(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<bool> {
    let (reader, writer) = pipe().unwrap();
    visit(left, CmdMeta::pipe_out(writer))?;
    visit(right, stdio.new_in(reader))
}

fn visit_simple(mut simple: Simple, stdio: CmdMeta) -> Option<bool> {
    reconcile_io(&mut simple, stdio);
    match &simple.cmd[..] {
        "exit" => exit(0),
        "cd" => {
            let root = Path::new(simple.args.get(0).map_or("/", |x| x));
            if let Err(e) = env::set_current_dir(&root) {
                eprintln!("rush: {}", e);
            }
            Some(true)
        }
        command => {
            let mut cmd = Command::new(command);
            cmd.args(&simple.args);

            cmd.stdin((simple.stdin).borrow_mut().get_stdin()?);
            cmd.stdout((simple.stdout).borrow_mut().get_stdout()?);
            cmd.stderr((simple.stderr).borrow_mut().get_stderr()?);

            match cmd.status() {
                Ok(child) => Some(child.success()),
                Err(e) => {
                    eprintln!("rush: {}", e);
                    None
                }
            }
        }
    }
}

// Takes the stdio and if stdio has priority, replaces stdout/stdin with it.
fn reconcile_io(simple: &mut Simple, stdio: CmdMeta) {
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

// How do I test this module?
