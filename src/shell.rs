use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub struct Shell;

impl Shell {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self) -> io::Result<()> {
        loop {
            print!("$ ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Unable to read line");

            let line = input.trim_end_matches(['\n', '\r']);
            let parts = match parse_line(line) {
                Ok(parts) => parts,
                Err(err) => {
                    eprintln!("parse error: {}", err);
                    continue;
                }
            };

            if parts.is_empty() {
                continue;
            }

            let command = parts[0].as_str();

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
                    let query = parts[1].as_str();

                    let builtin = ["exit", "echo", "type", "pwd", "cd"];

                    if builtin.contains(&query) {
                        println!("{} is a shell builtin", query);
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
                            Err(e) => {
                                eprintln!("cd: unable to read PATH environment variable: {}", e)
                            }
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

                    let args = &parts[1..];
                    let full_path = args.join(" ");
                    let path = Path::new(&full_path);
                    let path_str = full_path.as_str();
                    if path_str == "~" || path_str.starts_with("~/") {
                        match env::var("HOME") {
                            Ok(val) => {
                                let expanded = if path_str == "~" {
                                    val.clone()
                                } else {
                                    path_str.replacen("~", &val, 1)
                                };

                                if let Err(e) = env::set_current_dir(&expanded) {
                                    eprintln!("cd: {} {}", &expanded, e)
                                }
                            }
                            Err(e) => {
                                eprintln!("cd: unable to read HOME environment variable: {}", e)
                            }
                        }
                    } else if !path.exists() {
                        println!("cd: {}: no such file or directory", full_path);
                    } else if !path.is_dir() {
                        println!("cd: {}: not a directory", full_path);
                    } else if let Err(e) = env::set_current_dir(&full_path) {
                        eprintln!("cd: {} {}", path.display(), e);
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
                                println!("{}: command not found", command);
                            }
                        }
                        Err(e) => println!("couldn't find val {}", e),
                    }
                }
            }
        }
        Ok(())
    }
}

fn parse_line(input: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;
    let mut token_started = false;

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) => {
                if ch == active {
                    quote = None;
                    token_started = true;
                } else if ch == '\\' && active == '"' {
                    match chars.next() {
                        Some(next) => {
                            current.push(next);
                            token_started = true;
                        }
                        None => return Err("unterminated escape".to_string()),
                    }
                } else {
                    current.push(ch);
                    token_started = true;
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    token_started = true;
                }
                '\\' => match chars.next() {
                    Some(next) => {
                        current.push(next);
                        token_started = true;
                    }
                    None => return Err("unterminated escape".to_string()),
                },
                ch if ch.is_whitespace() => {
                    if token_started {
                        parts.push(std::mem::take(&mut current));
                        token_started = false;
                    }
                }
                _ => {
                    current.push(ch);
                    token_started = true;
                }
            },
        }
    }

    if quote.is_some() {
        return Err("unterminated quote".to_string());
    }

    if token_started {
        parts.push(current);
    }

    Ok(parts)
}
