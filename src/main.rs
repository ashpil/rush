use rush::lexer::Lexer;
use rush::parser::Parser;
use rush::runner::Runner;
use rush::helpers::Shell;
use std::env;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut args = env::args();
    args.next();

    let shell = Rc::new(RefCell::new(Shell::new(args.last())));
    let runner = Runner::new(Rc::clone(&shell));

    loop {
        let input = shell.borrow_mut().next();
        if let Some(line) = input {
            let lexer = Lexer::new(&line, Rc::clone(&shell));
            let mut parser = Parser::new(lexer, Rc::clone(&shell));
            match parser.get() {
                Ok(command) => {
                    #[cfg(debug_assertions)] // Only include when not built with `--release` flag
                    println!("\u{001b}[34m{:#?}\u{001b}[0m", command);

                    runner.execute(command);
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        } else {
            if shell.borrow().is_interactive() {
                println!();
            }
            break;
        }
    }
}
