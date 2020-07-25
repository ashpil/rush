use rush::lexer::Lexer;
use rush::runner::Runner;
use rush::parser::Parser;
use std::io::{stdin, stdout, Write};

fn main() {
    loop {
        print!("> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let lexer = Lexer::new(&input);

        let mut parser = Parser::new(lexer); 
        let command = parser.get();
        println!("[Main] Command: {:?}", command);

        let runner = Runner::new(command);
        runner.execute();
    }
}
