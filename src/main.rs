use std::io::{self, Write};
use std::env;
use std::path::Path;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

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
    
    let command = parts[0];
    
    match command {
        "exit" => break,
        "echo" => {
            let args = &parts[1..];
            println!("{}", args.join(" "));
        },
        "type" => {
            if parts.len() < 2 {
                continue;
            }
            let query = &parts[1];
            
            let builtin = ["exit", "echo", "type"];
            
            if builtin.contains(query) {
                println!("{} is a rshell builtin", query);
            } else {
                match env::var("PATH") {
                    Ok(val) => {
                       let mut found = false;
                       for dir in val.split(":") {
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
        _ =>  {
            if command.contains("/") {
                Command::new(command)
                    .args(&parts[1..])
                    .status()
                    .unwrap();
                
                continue;
            }
            
            match env::var("PATH") {
                Ok(val) => {
                    let mut found = false;
                   for dir in val.split(":") {
                       let full_path = Path::new(&dir).join(command);
                       
                       if !full_path.exists() {
                           continue;
                       }
                       
                       let metadata = match fs::metadata(&full_path) {
                           Ok(m) => m,
                           Err(_) => continue
                       };
                       
                       let mode = metadata.permissions().mode();
                       
                       if mode & 0o111 != 0 {
                           Command::new(full_path)
                               .args(&parts[1..])
                               .status()
                               .unwrap();
                           found = true;
                           break;
                       }
                   } 
                   if !found {
                       println!("{}: command not found", command);
                   }
                },
                Err(e) => println!("Coudn't find val {}", e)
            }
        }
    }
    
    }
}