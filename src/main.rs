mod shell;

fn main() {
    if let Err(err) = shell::Shell::new().run() {
        eprintln!("rshell: {}", err);
    }
}
