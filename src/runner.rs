use crate::parser::{Cmd, Fd, Simple};
use os_pipe::{pipe, PipeReader, PipeWriter};
use std::env;
use std::path::Path;
use std::process::{exit, Child, Command};


// This is useful to keep track of what each command does with its STDs
// and what the status is.
#[derive(Debug)]
struct CmdMeta {
    stdin: Option<PipeReader>,
    stdout: Option<PipeWriter>,
    success: Option<bool>,
}

impl CmdMeta {
    fn inherit() -> CmdMeta {
        CmdMeta {
            stdin: None,
            stdout: None,
            success: None,
        }
    }

    fn pipe_out(writer: PipeWriter) -> CmdMeta {
        CmdMeta {
            stdin: None,
            stdout: Some(writer),
            success: None,
        }
    }

    fn new_in(self, reader: PipeReader) -> CmdMeta {
        CmdMeta {
            stdin: Some(reader),
            stdout: self.stdout,
            success: self.success,
        }
    }

    fn from(mut child: Child) -> CmdMeta {
        let success = Some(child.wait().unwrap().success());
        CmdMeta {
            stdin: None,
            stdout: None,
            success,
        }
    }

    fn set_success(self, success: bool) -> CmdMeta {
        CmdMeta {
            stdin: self.stdin,
            stdout: self.stdout,
            success: Some(success),
        }
    }

    fn success(&self) -> bool {
        if let Some(success) = self.success {
            success
        } else {
            false
        }
    }

    fn successful() -> CmdMeta {
        CmdMeta {
            stdin: None,
            stdout: None,
            success: Some(true),
        }
    }
}

pub fn execute(ast: Cmd) {
    visit(ast, CmdMeta::inherit());
}

fn visit(node: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
    match node {
        Cmd::Simple(simple) => visit_simple(simple, stdio),
        Cmd::Pipeline(cmd0, cmd1) => visit_pipe(*cmd0, *cmd1, stdio),
        Cmd::And(cmd0, cmd1) => visit_and(*cmd0, *cmd1, stdio),
        Cmd::Or(cmd0, cmd1) => visit_or(*cmd0, *cmd1, stdio),
        Cmd::Not(cmd) => visit_not(*cmd, stdio),
    }
}

fn visit_not(cmd: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
    let result = visit(cmd, stdio)?;
    let success = result.success();
    Some(result.set_success(!success))
}

fn visit_or(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
    let left = visit(left, CmdMeta::inherit())?;
    if !left.success() {
        visit(right, stdio)
    } else {
        Some(left)
    }
}

fn visit_and(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
    let left = visit(left, CmdMeta::inherit())?;
    if left.success() {
        visit(right, stdio)
    } else {
        Some(left)
    }
}

// This tells the left command to keep track of its stdout for later,
// and then takes that and puts it into the stdin of the right command.

// Due to the stdio arg, we know whether this is the uppermost command in
// in the tree, and thus can tell whether we should pipe its output or not.
fn visit_pipe(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
    let (reader, writer) = pipe().unwrap();
    visit(left, CmdMeta::pipe_out(writer))?;
    visit(right, stdio.new_in(reader))
}

fn visit_simple(mut simple: Simple, stdio: CmdMeta) -> Option<CmdMeta> {
    reconcile_io(&mut simple, stdio);
    match &simple.cmd[..] {
        "exit" => exit(0),
        "cd" => {
            let root = Path::new(simple.args.get(0).map_or("/", |x| x));
            if let Err(e) = env::set_current_dir(&root) {
                eprintln!("rush: {}", e);
            }
            Some(CmdMeta::successful())
        }
        command => {
            let mut cmd = Command::new(command);
            cmd.args(&simple.args);

            cmd.stdin((simple.stdin).borrow_mut().get_stdin()?);
            cmd.stdout((simple.stdout).borrow_mut().get_stdout()?);
            cmd.stderr((simple.stderr).borrow_mut().get_stderr()?);

            match cmd.spawn() {
                Ok(child) => Some(CmdMeta::from(child)),
                Err(e) => {
                    eprintln!("rush: {}", e);
                    None
                }
            }
        }
    }
}

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
