use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::runner::Runner;
use std::io::{stdin, stdout, Write};

// TODO EVERYWHERE: Actual error handling
fn main() {
    loop {
        print!("~> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let lexer = Lexer::new(&input);

        let mut parser = Parser::new(lexer); 
        match parser.get() {
            Ok(command) => {
                // Some helpful debug printing
                println!("\u{001b}[34m[Main]\u{001b}[0m Command: {:?}", command);

                let runner = Runner::new(command);

                // Colors just make it easier to notice stuff immediately for debug
                println!("\u{001b}[33mCommand output:\u{001b}[0m");
                runner.execute();
            }, 
            Err(e) => {
                eprintln!("{}", e);
            },
        }
    }
}
