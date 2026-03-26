# rshell

a minimal unix-like shell written in rust.

this project is a hands-on learning journey, built alongside my growing understanding of rust and systems programming.

the goal is to explore how shells work under the hood and to learn system design, advanced programming practices, and real-world computing concepts by building them from scratch. this includes how a shell interacts with the operating system and how software is actually executed on a computer.

beyond technical depth, there is something deeply satisfying about understanding a tool you use every day.

if you are reading this early version of the readme, it means the project is still in an active learning phase. over time, as the implementation matures, this repository will evolve into a more complete and polished open-source project, and the documentation will grow with it.

current capabilities:

- builtins for `cd`, `echo`, `exit`, `history`, `pwd`, and `type`
- quoted and escaped argument parsing for commands like `echo "hello world"`
- `cd ~`, `cd ~/path`, and `cd -`
- executable lookup through `PATH`
- a configurable prompt via the `RSHELL_PROMPT` environment variable

run it with:

```bash
cargo run
```

test it with:

```bash
cargo test
```

until then, let’s get rusty.
