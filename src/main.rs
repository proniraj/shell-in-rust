use std::io::{self, Write};

// -> io::Result<()>
fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();

        if command.to_string() == "exit" {
            break;
        }

        io::stdin()
            .read_line(&mut command)
            .expect("Failed to read line");

        print!("{}: command not found\n", command.trim());
    }

    // Ok(())
}
