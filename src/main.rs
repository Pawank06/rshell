use std::io::{self, Write};

fn main() {
    loop {    
    print!("$ ");
    io::stdout()
        .flush().unwrap();
    
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input).expect("Unable to read line");
    
    let parts: Vec<&str> = input.split_whitespace().collect();
    
    if parts.is_empty() {
        continue;
    }
    
    match parts[0] {
        "exit" => break,
        "echo" => {
            let args = &parts[1..];
            println!("{}", args.join(" "));
        },
        "type" => {
            let builtin = ["exit", "echo"];
            let query = &parts[1];
            
            if builtin.contains(query) {
                println!("{} is a rshell builtin", query);
            } else {
                println!("{} not found", query);
            }
        },
        _ =>  println!("{}: command not found", input.trim()),
    }
    
    }
}