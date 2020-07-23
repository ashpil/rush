use rush::lexer::Lexer;
use rush::parser::{Parser, AST};
use std::io::{stdin, stdout, Write};
use std::process::Command;

fn main() {
    loop {
        print!("> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let lexer = Lexer::new(&input);
        let mut parser = Parser::new(lexer); 
        let command = parser.gen();
        println!("Command in main: {:?}", command);

        if let AST::Basic(vec) = command {
            println!("{:?}", vec);
            Command::new(&vec[0])
                .args(&vec[1..])
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
        }
    }
}
