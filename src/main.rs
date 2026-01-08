use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Unable to read line");

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
            }
            "type" => {
                if parts.len() < 2 {
                    continue;
                }
                let query = &parts[1];

                let builtin = ["exit", "echo", "type", "pwd"];

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
                                    Err(_) => continue,
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
                        }
                        Err(e) => eprintln!("Couldn't read PATH environment variable: {}", e),
                    }
                }
            }
            "pwd" => match env::current_dir() {
                Ok(val) => println!("{}", val.display()),
                Err(e) => eprintln!("pwd: {}", e),
            },
            "cd" => {
                if parts.len() < 2 {
                    continue;
                }

                let query = parts[1];

                if query.starts_with("/") {
                    let args = &parts[1..];
                    let full_path = args.join(" ");

                    let path = Path::new(&full_path);
                    if !path.exists() {
                        println!("cd: {}: No such file or directory", full_path);
                    } else if !path.is_dir() {
                        println!("cd: {}: Not a directory", full_path);
                    } else {
                        if let Err(e) = env::set_current_dir(&full_path) {
                            eprintln!("cd: {} {}", path.display(), e);
                        }
                    }
                } else {
                    let args = &parts[1..];
                    let full_path = args.join(" ");
                    println!("cd: {}: No such file or directory", full_path);
                }
            }
            _ => {
                if command.contains("/") {
                    match Command::new(command).args(&parts[1..]).status() {
                        Ok(_) => {}
                        Err(e) => eprintln!("{}: {}", command, e),
                    };

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
                                Err(_) => continue,
                            };

                            let mode = metadata.permissions().mode();

                            if mode & 0o111 != 0 {
                                match Command::new(&full_path)
                                    .arg0(command)
                                    .args(&parts[1..])
                                    .status()
                                {
                                    Ok(_) => {}
                                    Err(err) => eprintln!("{}: {}", command, err),
                                }
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            println!("{}: Command not found", command);
                        }
                    }
                    Err(e) => println!("Couldn't find val {}", e),
                }
            }
        }
    }
}
