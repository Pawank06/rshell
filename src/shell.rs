use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Shell {
    history: Vec<String>,
    previous_dir: Option<PathBuf>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            previous_dir: None,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        loop {
            print!("{}", prompt());
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

            self.history.push(line.to_string());

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

                    let builtin = ["exit", "echo", "type", "pwd", "cd", "history"];

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
                "history" => {
                    for (index, entry) in self.history.iter().enumerate() {
                        println!("{:>4}  {}", index + 1, entry);
                    }
                }
                "cd" => {
                    if let Err(e) = self.change_dir(&parts) {
                        eprintln!("cd: {}", e);
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

    fn change_dir(&mut self, parts: &[String]) -> io::Result<()> {
        let target = parts.get(1).map(|value| value.as_str()).unwrap_or("~");
        let next_dir = if target == "-" {
            match self.previous_dir.clone() {
                Some(path) => {
                    println!("{}", path.display());
                    path
                }
                None => {
                    eprintln!("cd: OLDPWD not set");
                    return Ok(());
                }
            }
        } else {
            expand_path(target)?
        };

        if !next_dir.exists() {
            eprintln!("cd: {}: no such file or directory", next_dir.display());
            return Ok(());
        }

        if !next_dir.is_dir() {
            eprintln!("cd: {}: not a directory", next_dir.display());
            return Ok(());
        }

        let current_dir = env::current_dir()?;
        if let Err(err) = env::set_current_dir(&next_dir) {
            eprintln!("cd: {}: {}", next_dir.display(), err);
            return Ok(());
        }

        self.previous_dir = Some(current_dir);
        Ok(())
    }
}

fn prompt() -> String {
    env::var("RSHELL_PROMPT").unwrap_or_else(|_| "$ ".to_string())
}

fn expand_path(input: &str) -> io::Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(expand_path_from_home(input, &home))
}

fn expand_path_from_home(input: &str, home: &Path) -> PathBuf {
    if input == "~" {
        return home.to_path_buf();
    }

    if let Some(rest) = input.strip_prefix("~/") {
        return home.join(rest);
    }

    PathBuf::from(input)
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

#[cfg(test)]
mod tests {
    use super::parse_line;

    #[test]
    fn parse_line_splits_whitespace() {
        assert_eq!(
            parse_line("echo hello world").unwrap(),
            vec!["echo", "hello", "world"]
        );
    }

    #[test]
    fn parse_line_keeps_quoted_segments() {
        assert_eq!(
            parse_line("echo \"hello world\" 'from rust'").unwrap(),
            vec!["echo", "hello world", "from rust"]
        );
    }

    #[test]
    fn parse_line_handles_escaped_spaces() {
        assert_eq!(
            parse_line("touch hello\\ world.txt").unwrap(),
            vec!["touch", "hello world.txt"]
        );
    }

    #[test]
    fn parse_line_rejects_unterminated_quotes() {
        assert!(parse_line("echo \"hello").is_err());
    }

    #[test]
    fn parse_line_preserves_empty_quoted_arguments() {
        assert_eq!(
            parse_line("echo \"\" '' done").unwrap(),
            vec!["echo", "", "", "done"]
        );
    }
}
