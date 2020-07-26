use crate::parser::Cmd;
use std::process::{Child, Command, Stdio};

// This is useful to keep track of what each command does with its STDs.
struct InOut {
    stdout: Option<Stdio>,
    stdin: Option<Stdio>,
}

impl InOut {
    fn inherit() -> InOut {
        InOut {
            stdout: None,
            stdin: None,
        }
    }

    fn pipe_out() -> InOut {
        InOut {
            stdout: Some(Stdio::piped()),
            stdin: None,
        }
    }

    fn new_in(self, stdin: Stdio) -> InOut {
        InOut {
            stdout: self.stdout,
            stdin: Some(stdin),
        }
    }
}

// Not sure if I need this to actually be a struct, this might just be OOP creeping into my code.
pub struct Runner {
    ast: Cmd,
}

impl Runner {
    pub fn new(ast: Cmd) -> Runner {
        Runner { ast }
    }

    pub fn execute(self) {
        Self::visit(self.ast, InOut::inherit()).wait().unwrap();
    }

    fn visit(node: Cmd, stdio: InOut) -> Child {
        match node {
            Cmd::Simple(cmd) => Self::visit_simple(cmd, stdio),
            Cmd::Pipeline(cmd0, cmd1) => Self::visit_pipe(*cmd0, *cmd1, stdio),
            _ => Self::visit_simple(vec!["ls".to_string()], stdio), // This is a workaround because I'm lazy, this should return a result
        }
    }

    // This tells the left command to keep track of its stdout for later,
    // and then takes that and puts it into the stdin of the right command.

    // Due to the stdio arg, we know whether this is the uppermost command in
    // in the tree, and thus can tell whether we should pipe its output or not.
    fn visit_pipe(left: Cmd, right: Cmd, stdio: InOut) -> Child {
        let left = Self::visit(left, InOut::pipe_out());
        let stdin = Stdio::from(left.stdout.unwrap());
        Self::visit(right, stdio.new_in(stdin))
    }

    // We add the relevant stdios if they are not None.
    fn visit_simple(cmd: Vec<String>, stdio: InOut) -> Child {
        let mut child = Command::new(&cmd[0]);
        if let Some(stdout) = stdio.stdout {
            child.stdout(stdout);
        }
        if let Some(stdin) = stdio.stdin {
            child.stdin(stdin);
        }
        child.args(&cmd[1..]).spawn().unwrap()
    }
}
