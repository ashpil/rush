use regex::Regex;
use std::collections::BTreeMap;
use std::process::exit as exit_program;
use std::env;
use std::rc::Rc;
use std::cell::RefCell;
use crate::helpers::Shell;

// Unless specified otherwise, if provided multiple arguments while only
// accepting one, these use the first argument. Dash does this as well.  

pub fn alias(
    // Aliases can be added and then printed in the same command
    aliases: &mut BTreeMap<String, String>,
    args: Vec<String>,
) -> bool {
    if args.len() == 0 {
        for (lhs, rhs) in aliases {
            println!("alias {}='{}'", lhs, rhs);
        }
        true
    } else {
        let mut success = true;
        let assignment_re = Regex::new(r"^(\w+)=(.*)").unwrap();
        for arg in args {
            if assignment_re.is_match(&arg) {
                let caps = assignment_re.captures(&arg).unwrap();
                let lhs = &caps[1];
                let rhs = &caps[2];
                aliases.insert(lhs.to_string(), rhs.to_string());
            } else if aliases.contains_key(&arg) {
                println!("alias {}='{}'", arg, aliases[&arg]);
            } else {
                eprintln!("rush: alias: {}: not found", arg);
                success = false;
            }
        }
        success
    }
}

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

