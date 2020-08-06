use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::runner::execute;
use rush::helpers::Mode;
use std::env;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut args = env::args();
    args.next();

    let mode = Rc::new(RefCell::new(Mode::new(args.last())));

    loop {
        let input = mode.borrow_mut().next();
        if let Some(line) = input {
            let lexer = Lexer::new(&line, Rc::clone(&mode));
            let mut parser = Parser::new(lexer, Rc::clone(&mode));
            match parser.get() {
                Ok(command) => {
                    #[cfg(debug_assertions)] // Only include when not built with `--release` flag
                    println!("\u{001b}[34m{:#?}\u{001b}[0m", command);

                    execute(command);
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        } else {
            if mode.borrow().is_interactive() {
                println!();
            }
            break;
        }
    }
}
