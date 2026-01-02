use std::io::{self, Write};
use std::env;
use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;

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
            if parts.len() < 2 {
                continue;
            }
            
            let builtin = ["exit", "echo", "type"];
            let query = &parts[1];
            
            if builtin.contains(query) {
                println!("{} is a rshell builtin", query);
            } else {
                match env::var("PATH") {
                    Ok(val) => {
                       let paths: Vec<&str> = val.split(":").collect();
                       let mut found = false;
                       for dir in paths {
                           let full_path = Path::new(dir).join(query);
                           if !full_path.exists() {
                               continue;
                           }
                           
                           let metadata = match fs::metadata(&full_path) {
                               Ok(m) => m,
                               Err(_) => continue
                           };
                           
                           let mode = metadata.permissions().mode();
                           
                           if mode & 0o111 != 0 {
                               println!("{} is {}", query, full_path.display());
                               found = true;
                               break;
                           }                          
                       }
                       
                       if !found {
                           println!("{}: not found", query);
                       }
                    },
                    Err(e) => println!("Couldn't find val {}", e)
                }
            }
        },
        _ =>  println!("{}: command not found", input.trim()),
    }
    
    }
}