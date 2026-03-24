use std::env;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

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
            io::stdout().flush()?;

            let mut input = String::new();
            if io::stdin().read_line(&mut input)? == 0 {
                println!();
                break;
            }

            let line = input.trim_end_matches(['\n', '\r']);
            if line.trim().is_empty() {
                continue;
            }

            self.history.push(line.to_string());

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

            if self.try_builtin(&parts)? {
                continue;
            }

            run_external(&parts);
        }

        Ok(())
    }

    fn try_builtin(&mut self, parts: &[String]) -> io::Result<bool> {
        match parts[0].as_str() {
            "exit" => {
                let code = parts
                    .get(1)
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0);
                process::exit(code);
            }
            "echo" => {
                println!("{}", parts[1..].join(" "));
                Ok(true)
            }
            "pwd" => {
                println!("{}", env::current_dir()?.display());
                Ok(true)
            }
            "history" => {
                if should_clear_history(parts.get(1).map(String::as_str)) {
                    self.history.clear();
                    return Ok(true);
                }
                for (index, entry) in
                    slice_history_entries(&self.history, parts.get(1).map(String::as_str))
                {
                    println!("{:>4}  {}", index + 1, entry);
                }
                Ok(true)
            }
            "help" => {
                for line in help_lines(parts.get(1).map(String::as_str)) {
                    println!("{}", line);
                }
                Ok(true)
            }
            "type" => {
                for query in parts.iter().skip(1) {
                    if builtin_names().contains(&query.as_str()) {
                        println!("{} is a shell builtin", query);
                    } else if let Some(path) = find_executable(query) {
                        println!("{} is {}", query, path.display());
                    } else {
                        println!("{}: not found", query);
                    }
                }
                Ok(true)
            }
            "cd" => {
                self.change_dir(parts)?;
                Ok(true)
            }
            _ => Ok(false),
        }
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
    let template = env::var("RSHELL_PROMPT").unwrap_or_else(|_| "$ ".to_string());
    let cwd = env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    render_prompt(&template, &cwd)
}

fn builtin_names() -> &'static [&'static str] {
    &["cd", "echo", "exit", "help", "history", "pwd", "type"]
}

fn help_lines(topic: Option<&str>) -> Vec<String> {
    match topic {
        Some("cd") => vec!["cd [dir]".to_string(), "change the current directory".to_string()],
        Some("echo") => vec!["echo [args...]".to_string(), "print arguments to stdout".to_string()],
        Some("exit") => vec!["exit [code]".to_string(), "exit the shell".to_string()],
        Some("help") => vec!["help [command]".to_string(), "show builtin command help".to_string()],
        Some("history") => vec![
            "history [n|clear]".to_string(),
            "print entered commands or clear the history".to_string(),
        ],
        Some("pwd") => vec!["pwd".to_string(), "print the current directory".to_string()],
        Some("type") => vec!["type [command]".to_string(), "describe how a command is resolved".to_string()],
        Some(other) => vec![format!("{}: no builtin help available", other)],
        None => vec![
            "builtins: cd, echo, exit, help, history, pwd, type".to_string(),
            "use `help <command>` for details".to_string(),
        ],
    }
}

fn should_clear_history(arg: Option<&str>) -> bool {
    matches!(arg, Some("clear" | "-c"))
}

fn render_prompt(template: &str, cwd: &str) -> String {
    template.replace("{cwd}", cwd)
}

fn slice_history_entries(history: &[String], limit: Option<&str>) -> Vec<(usize, String)> {
    let limit = limit
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(history.len());
    let start = history.len().saturating_sub(limit);
    history
        .iter()
        .enumerate()
        .skip(start)
        .map(|(index, entry)| (index, entry.clone()))
        .collect()
}

fn find_executable(command: &str) -> Option<PathBuf> {
    if command.contains('/') {
        let path = PathBuf::from(command);
        return is_executable(&path).then_some(path);
    }

    let path_value = env::var_os("PATH")?;
    find_executable_in_paths(command, env::split_paths(&path_value))
}

fn find_executable_in_paths<I>(command: &str, paths: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    for dir in paths {
        let candidate = dir.join(command);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return false,
    };

    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

fn run_external(parts: &[String]) {
    let command = &parts[0];
    let executable = match find_executable(command) {
        Some(path) => path,
        None => {
            eprintln!("{}: command not found", command);
            return;
        }
    };

    if let Err(err) = Command::new(&executable)
        .arg0(command)
        .args(&parts[1..])
        .status()
    {
        eprintln!("{}: {}", command, err);
    }
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
    use super::{
        expand_path_from_home, find_executable_in_paths, help_lines, parse_line, render_prompt,
        should_clear_history, slice_history_entries,
    };
    use std::env;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn expand_path_expands_home_directory() {
        assert_eq!(
            expand_path_from_home("~", Path::new("/tmp/rshell-home")),
            PathBuf::from("/tmp/rshell-home")
        );
        assert_eq!(
            expand_path_from_home("~/docs", Path::new("/tmp/rshell-home")),
            PathBuf::from("/tmp/rshell-home/docs")
        );
    }

    #[test]
    fn find_executable_locates_binary_from_path() {
        let base = env::temp_dir().join(unique_name("rshell-test"));
        let binary = base.join("demo-command");

        fs::create_dir_all(&base).unwrap();
        fs::write(&binary, "echo demo").unwrap();

        let mut permissions = fs::metadata(&binary).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary, permissions).unwrap();

        assert_eq!(
            find_executable_in_paths("demo-command", [base.clone()]),
            Some(binary.clone())
        );

        fs::remove_file(&binary).unwrap();
        fs::remove_dir(&base).unwrap();
    }

    #[test]
    fn help_lines_lists_builtins() {
        assert_eq!(
            help_lines(None),
            vec![
                "builtins: cd, echo, exit, help, history, pwd, type",
                "use `help <command>` for details"
            ]
        );
    }

    #[test]
    fn slice_history_entries_returns_tail() {
        let history = vec![
            "echo one".to_string(),
            "echo two".to_string(),
            "echo three".to_string(),
        ];
        assert_eq!(
            slice_history_entries(&history, Some("2")),
            vec![
                (1, "echo two".to_string()),
                (2, "echo three".to_string())
            ]
        );
    }

    #[test]
    fn should_clear_history_recognizes_clear_flags() {
        assert!(should_clear_history(Some("clear")));
        assert!(should_clear_history(Some("-c")));
        assert!(!should_clear_history(Some("2")));
    }

    #[test]
    fn render_prompt_replaces_cwd_token() {
        assert_eq!(render_prompt("{cwd} $ ", "/tmp/demo"), "/tmp/demo $ ");
    }

    fn unique_name(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}-{}-{}", prefix, std::process::id(), nanos)
    }
}
