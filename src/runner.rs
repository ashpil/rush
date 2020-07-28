use crate::parser::Cmd;
use std::process::{Child, Command, Stdio, exit};
use std::path::Path;
use std::env;

// This is useful to keep track of what each command does with its STDs
// and what the status is.
#[derive(Debug)]
struct CmdMeta {
    stdout: Option<Stdio>,
    stdin: Option<Stdio>,
    success: Option<bool>,
}

impl CmdMeta {
    fn inherit() -> CmdMeta {
        CmdMeta {
            stdout: None,
            stdin: None,
            success: None,
        }
    }

    fn pipe_out() -> CmdMeta {
        CmdMeta {
            stdout: Some(Stdio::piped()),
            stdin: None,
            success: None,
        }
    }

    fn from(mut child: Child) -> CmdMeta {
        let success = Some(child.wait().unwrap().success());
        let stdout = if child.stdout.is_some() {
            Some(Stdio::from(child.stdout.unwrap()))
        } else {
            None
        };
        let stdin = if child.stdin.is_some() {
            Some(Stdio::from(child.stdin.unwrap()))
        } else {
            None
        };
        CmdMeta {
            stdout,
            stdin,
            success,
        }
    }

    fn new_in(self, stdin: Stdio) -> CmdMeta {
        CmdMeta {
            stdout: self.stdout,
            stdin: Some(stdin),
            success: None,
        }
    }

    fn set_success(self, success: bool) -> CmdMeta {
        CmdMeta {
            stdout: self.stdout,
            stdin: self.stdin,
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
}

// Not sure if I need this to actually be a struct, this might just be OOP bs creeping into my code.
pub struct Runner {
    ast: Cmd,
}

impl Runner {
    pub fn new(ast: Cmd) -> Runner {
        Runner { ast }
    }

    pub fn execute(self) {
        Self::visit(self.ast, CmdMeta::inherit());
    }

    fn visit(node: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
        match node {
            Cmd::Simple(vec) => Self::visit_simple(vec, stdio),
            Cmd::Pipeline(cmd0, cmd1) => Self::visit_pipe(*cmd0, *cmd1, stdio),
            Cmd::And(cmd0, cmd1) => Self::visit_and(*cmd0, *cmd1, stdio),
            Cmd::Or(cmd0, cmd1) => Self::visit_or(*cmd0, *cmd1, stdio),
            Cmd::Not(cmd) => Self::visit_not(*cmd, stdio),
            _ => unimplemented!(),
        }
    }

    fn visit_not(cmd: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
        let result = Self::visit(cmd, stdio)?;
        let success = result.success();
        Some(result.set_success(!success))
    }

    fn visit_or(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
        let left = Self::visit(left, CmdMeta::inherit())?;
        if !left.success() {
            Self::visit(right, stdio)
        } else {
            Some(left)
        }
    }

    fn visit_and(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<CmdMeta> {
        let left = Self::visit(left, CmdMeta::inherit())?;
        if left.success() {
            Self::visit(right, stdio)
        } else {
            Some(left)
        }
    }

    // This tells the left command to keep track of its stdout for later,
    // and then takes that and puts it into the stdin of the right command.

    // Due to the stdio arg, we know whether this is the uppermost command in
    // in the tree, and thus can tell whether we should pipe its output or not.
    fn visit_pipe(left: Cmd, right: Cmd, stdio: CmdMeta) -> Option<CmdMeta> { 
        let left = Self::visit(left, CmdMeta::pipe_out())?;
        let stdin = Stdio::from(left.stdout.unwrap());
        Self::visit(right, stdio.new_in(stdin))
    }

    // We add the relevant stdios if they are not None.
    fn visit_simple(cmd: Vec<String>, stdio: CmdMeta) -> Option<CmdMeta> {
        match &cmd[0][..] {
            "exit" => exit(0),
            "cd" => {
                let root = Path::new(cmd.get(1).map_or("/", |x| x));
                if let Err(e) = env::set_current_dir(&root) {
                    eprintln!("Rush error: {}", e);
                }
                Some(stdio.set_success(true))
            },
            command => {
                let mut child = Command::new(command);
                if let Some(stdout) = stdio.stdout {
                    child.stdout(stdout);
                }
                if let Some(stdin) = stdio.stdin {
                    child.stdin(stdin);
                }
                // No idea why this doesn't error when the vector has only 1 item, but
                // I guess it's neat.
                match child.args(&cmd[1..]).spawn() {
                    Ok(child) => Some(CmdMeta::from(child)),
                    Err(e) => {
                        eprintln!("Rush error: {}", e);
                        None
                    },
                }
            },
        }
    }
}
