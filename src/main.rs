use std::io::{self, Write};

fn main() -> io::Result<()> {
    let shell_builtin_commands = ["echo", "exit", "type"];

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();

        io::stdin()
            .read_line(&mut command)
            .expect("Failed to read line");

        if command.trim() == "exit" {
            break;
        } else if command.trim().starts_with("echo") {
            println!("{}", &command.trim()[5..]);
        } else if command.trim().starts_with("type") {
            let mut found: bool = false;
            for shell_builtin_command in shell_builtin_commands {
                if shell_builtin_command == &command.trim()[5..] {
                    found = true;
                    break;
                }
            }
            if found {
                println!("{} is a shell builtin", &command.trim()[5..]);
            } else {
                println!("{}: not found", &command.trim()[5..]);
            }
        } else {
            println!("{}: command not found", command.trim());
        }
    }

    Ok(())
}
