use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::runner::execute;
use std::io::{stdin, stdout, Write};

fn main() {
    let mut stdout = stdout();
    let stdin = stdin();
    let mut input = String::new();

    loop {
        print!("~> ");
        stdout.flush().unwrap();

        input.clear(); // `read_line` appends, not replaces, so need to clear the string first
        stdin.read_line(&mut input).unwrap();

        let lexer = Lexer::new(&input);

        let mut parser = Parser::new(lexer); 
        match parser.get() {
            Ok(command) => {
                #[cfg(debug_assertions)] // Only include when not built with `--release` flag
                println!("\u{001b}[34m[Main]\u{001b}[0m Command: {:?}", command);

                execute(command);
            }, 
            Err(e) => {
                eprintln!("{}", e);
            },
        }
    }
}
