use std::process::exit as exit_program;
use std::env;

// Unless specified otherwise, if provided multiple arguments while only
// accepting one, these use the last argument. Dash seems to use the first 
// one, while bash errors. I think it doesn't matter much and popping seems
// easiest, so for now I'm going with that 

pub fn exit(mut args: Vec<String>) -> bool {
    match args.pop().map_or(Ok(0), |x| x.parse::<i32>()) {
        Ok(n) => {
            exit_program(n);
        },
        Err(e) => {
            eprintln!("rush: {}", e);
            false
        },
    }
}

pub fn cd(mut args: Vec<String>) -> bool {
    let option = args.pop();
    if let Err(e) = env::set_current_dir(option.as_deref().unwrap_or("/")) {
        eprintln!("rush: {}", e);
        false
    } else {
        true
    }
}
