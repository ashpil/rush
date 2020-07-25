use crate::parser::Cmd;
use std::io::{Error, ErrorKind};
use std::process::{Child, Command, Stdio};

pub struct Runner {
    ast: Cmd,
}

impl Runner {
    pub fn new(ast: Cmd) -> Runner {
        Runner { ast }
    }

    pub fn execute(self) {
        match self.ast {
            Cmd::Simple(cmd) => Self::run(&cmd, None, false).wait(),
            Cmd::Pipeline(cmd) => Self::pipe(cmd).wait(),
            _ => Err(Error::new(ErrorKind::Other, "oh no!")),
        }.unwrap();
    }

    pub fn run(cmd: &Vec<String>, pipe_in: Option<Stdio>, pipe_out: bool) -> Child {
        let stdin = if pipe_in.is_some() {
            pipe_in.unwrap()
        } else {
            Stdio::inherit()
        };
        let stdout = if pipe_out {
            Stdio::piped()
        } else {
            Stdio::inherit()
        };
        Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdin(stdin)
            .stdout(stdout)
            .spawn()
            .unwrap()
    }

    pub fn pipe(commands: Vec<Cmd>) -> Child {
        let mut commands = commands.iter().peekable();
        let mut previous = None;
        while let Some(Cmd::Simple(cmd)) = commands.next() {
            let stdin = previous.map_or(Stdio::inherit(), |output: Child| {
                Stdio::from(output.stdout.unwrap())
            });

            previous = Some(Self::run(cmd, Some(stdin), commands.peek().is_some()));
        }
        previous.unwrap()
    }
}
