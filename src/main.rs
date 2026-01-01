use std::io::{self, Write};

fn main() {
    loop {    
    print!("$ ");
    io::stdout()
        .flush().unwrap();
    
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input).expect("Unable to read line");
    
    let command = input.trim();
    
    match command {
        "exit" => break,
        _ =>  println!("{}: command not found", &command),
    }
    
    }
}