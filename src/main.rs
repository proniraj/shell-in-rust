use std::io::{self, Write};

fn main() -> io::Result<()> {
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
            print!("{}\n", &command.trim()[5..]);
            break;
        }

        print!("{}: command not found\n", command.trim());
    }

    Ok(())
}
