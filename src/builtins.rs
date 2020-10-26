use std::process::exit as exit_program;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use crate::helpers::Shell;

// Unless specified otherwise, if provided multiple arguments while only
// accepting one, these use the first argument. Dash does this as well.  

pub fn exit(args: Vec<String>) -> bool {
    match args.get(0).map_or(Ok(0), |x| x.parse::<i32>()) {
        Ok(n) => {
            exit_program(n);
        },
        Err(e) => {
            eprintln!("rush: {}", e);
            false
        },
    }
}

pub fn cd(args: Vec<String>) -> bool {
    let new_dir = args.into_iter().next().unwrap_or_else(|| env::var("HOME").unwrap());
    if let Err(e) = env::set_current_dir(new_dir) {
        eprintln!("rush: {}", e);
        false
    } else {
        true
    }
}

// Set very versetaile normally, this is just positional parameters for now
pub fn set(args: Vec<String>, shell: &Rc<RefCell<Shell>>) -> bool {
    shell.borrow_mut().set_pos(args);
    true
}

