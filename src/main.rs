use std::env;
use std::fs::{self};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn find_executable(command: &str) -> Option<PathBuf> {
    let env_paths = env::var_os("PATH")?;

    for env_path in env::split_paths(&env_paths) {
        let candidate = env_path.join(command);

        let metadata = match fs::symlink_metadata(&candidate) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if !metadata.is_file() {
            continue;
        }

        let permissions_mode = metadata.permissions().mode();
        if permissions_mode & 0o111 != 0 {
            return Some(candidate);
        }
    }

    None
}

fn type_command(arguments: &[&str]) {
    match arguments {
        [arg @ ("echo" | "exit" | "type" | "pwd")] => println!("{} is a shell builtin", arg),
        [arg @ ..] => match find_executable(&arg.join(" ")) {
            Some(path) => println!("{} is {}", arg.join(" "), path.to_str().unwrap()),
            None => println!("{}: not found", arg.join(" ")),
        },
    }
}

fn run_external_command(command_with_args: &[&str]) {
    match command_with_args {
        [command, args @ ..] => match find_executable(command) {
            Some(path) => {
                let mut cmd = Command::new(&path);

                cmd.args(args);

                match cmd.output() {
                    Ok(result) => {
                        if result.status.success() {
                            print!("{}", String::from_utf8_lossy(&result.stdout));
                        } else {
                            eprint!("{}", String::from_utf8_lossy(&result.stderr));
                        }
                    }
                    Err(error) => eprintln!("Error running command: {}", error),
                }
            }
            None => println!("{}: command not found", command),
        },
        _ => (),
    }
}

fn pwd_command() {
    match env::current_dir() {
        Ok(path) => {
            println!("{}", path.to_str().unwrap());
        }
        Err(_) => (),
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
            ["pwd"] => pwd_command(),
            external_command => run_external_command(external_command),
        }
    }
}
