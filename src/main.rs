use std::env;
use std::fs::{self};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;

fn find_executable(command: &str) -> Option<std::path::PathBuf> {
    match env::var_os("PATH") {
        Some(env_paths) => {
            for env_path in env::split_paths(&env_paths) {
                let joined_path = env_path.as_path().join(command);
                let path = joined_path.to_str().unwrap();

                match fs::symlink_metadata(path) {
                    Ok(metadata) => {
                        if metadata.is_dir() {
                            continue;
                        }

                        let permissions_mode = metadata.permissions().mode();
                        let can_execute = permissions_mode & 0o111 != 0;

                        if can_execute {
                            return Some(joined_path);
                        }
                    }
                    _ => (),
                }
            }
        }
        None => return None,
    }

    None
}

fn type_command(arguments: &[&str]) {
    match arguments {
        [arg @ ("echo" | "exit")] => println!("{} is a shell builtin", arg),
        [arg @ ..] => match find_executable(&arg.join(" ")) {
            Some(path) => println!("{} is {}", arg.join(" "), path.to_str().unwrap()),
            None => println!("{}: not found", arg.join(" ")),
        },
    }
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_input = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        let full_command: Vec<&str> = user_input.split_whitespace().collect();

        match full_command.as_slice() {
            [] => continue,
            ["exit"] => std::process::exit(0),
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["type", rest @ ..] => type_command(rest),
            _ => println!("{}: command not found", full_command[0]),
        }
    }
}
