use std::env;
use std::fs::{self};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
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

                cmd.arg0(command);
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

fn change_directory(path: &Path) {
    if !path.is_dir() {
        println!("cd: {}: No such file or directory", path.to_str().unwrap());
        return;
    }

    if env::set_current_dir(&path).is_ok() {
        return;
    }
}

fn cd_command(args: &[&str]) {
    match args {
        [] => println!("Please provide directory path"),
        [directory_path, _res @ ..] => {
            let resolved_path = match *directory_path {
                "~" => PathBuf::from(env::var_os("HOME").unwrap()),
                rest => PathBuf::from(rest),
            };
            change_directory(Path::new(&resolved_path))
        }
    };
}

fn resolve_quote(string: &String) -> Vec<&str> {
    let mut start_index: Option<usize> = None;
    let mut last_word_end_index: usize = 0;
    let mut args: Vec<&str> = Vec::new();

    for (index, character) in string.chars().enumerate() {
        match character {
            '\'' => match start_index {
                None => start_index = Some(index),
                Some(start) => {
                    args.push(&string[start + 1..index]);
                    start_index = None;
                    last_word_end_index = index + 1;
                }
            },
            ' ' => match start_index {
                None => {
                    args.push(&string[last_word_end_index..index]);
                    last_word_end_index = index + 1;
                }
                _ => (),
            },
            _ => (),
        }
    }

    args
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_input = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        let full_command = resolve_quote(&user_input);

        match full_command.as_slice() {
            [] => continue,
            ["exit"] => std::process::exit(0),
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["type", rest @ ..] => type_command(rest),
            ["pwd"] => pwd_command(),
            ["cd", rest @ ..] => cd_command(rest),
            external_command => run_external_command(external_command),
        }
    }
}
