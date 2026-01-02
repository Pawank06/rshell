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
        _ =>  println!("{}: command not found", input.trim()),
    }
    
    }
}