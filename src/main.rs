use std::io::{self, Write};

fn main() -> io::Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_input = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        let full_command: Vec<&str> = user_input.split_whitespace().collect();

        if full_command.len() < 1 {
            continue;
        }

        match full_command.as_slice() {
            ["exit"] => std::process::exit(0),
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["type", rest @ ("echo" | "exit" | "type")] => println!("{} is a shell builtin", rest),
            ["type", rest @ ..] => println!("{}: not found", rest.join(" ")),
            _ => println!("{}: command not found", full_command[0]),
        }
    }

    // Ok(())
}
