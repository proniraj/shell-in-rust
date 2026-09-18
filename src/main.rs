use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
trait FileContent {
    fn to_bytes(&self) -> Vec<u8>;
}

impl FileContent for &str {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl FileContent for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
}

impl FileContent for &[u8] {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_vec()
    }
}

impl FileContent for Vec<u8> {
    fn to_bytes(&self) -> Vec<u8> {
        self.clone()
    }
}

impl FileContent for &[&str] {
    fn to_bytes(&self) -> Vec<u8> {
        self.join("\n").into_bytes()
    }
}

impl FileContent for &[String] {
    fn to_bytes(&self) -> Vec<u8> {
        self.join("\n").into_bytes()
    }
}

impl<const N: usize> FileContent for [&str; N] {
    fn to_bytes(&self) -> Vec<u8> {
        self.join("\n").into_bytes()
    }
}

fn write_file<P, C>(path: P, content: C, append: bool) -> io::Result<()>
where
    P: AsRef<Path>,
    C: FileContent,
{
    let path = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut options = OpenOptions::new();
    options.create(true).write(true);

    if append {
        options.append(true);
    } else {
        options.truncate(true);
    }

    // Open file and write bytes
    let mut file = options.open(path)?;
    file.write_all(&content.to_bytes())?;

    Ok(())
}

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

                let mut write_stdout_to_file: Option<&str> = None;
                let mut write_stderr_to_file: Option<&str> = None;

                cmd.arg0(command);

                match args {
                    [args_and_path @ .., ">" | "1>", stdout_file_path] => {
                        cmd.args(args_and_path);
                        write_stdout_to_file = Some(*stdout_file_path);
                    }
                    [args_and_path @ .., "2>", stderr_file_path] => {
                        cmd.args(args_and_path);
                        write_stderr_to_file = Some(*stderr_file_path);
                    }

                    rest_args => {
                        cmd.args(rest_args);
                    }
                }

                match cmd.output() {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            match write_stdout_to_file {
                                Some(path) => {
                                    write_file(path, output.stdout.as_slice(), false).unwrap()
                                }
                                None => {
                                    io::stdout().write_all(&output.stdout).unwrap();

                                    if !output.stdout.ends_with(b"\n") {
                                        print!("\n");
                                    }
                                }
                            }
                        }

                        if !output.stderr.is_empty() {
                            match write_stderr_to_file {
                                Some(path) => {
                                    write_file(path, output.stderr.as_slice(), false).unwrap()
                                }
                                None => {
                                    io::stderr().write_all(&output.stderr).unwrap();

                                    if !output.stderr.ends_with(b"\n") {
                                        print!("\n");
                                    }
                                }
                            }
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

enum State {
    Inside,
    Outside,
}

#[derive(PartialEq)]
enum Quote {
    Single,
    Double,
    None,
}

fn command_tokenizer() -> Vec<String> {
    let mut state: State = State::Outside;
    let mut quote: Quote = Quote::Single;
    // backslash \ is used outside of quotes, it acts as an escape character
    let mut is_escaping: bool = false;

    let mut args: Vec<String> = Vec::new();
    let mut current_argument: String = String::new();

    let mut user_input = String::new();

    'outer: loop {
        user_input.clear();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        for char in user_input.chars() {
            match char {
                '\n' => {
                    match state {
                        State::Outside => {
                            // finiish arguments
                            if !current_argument.is_empty() {
                                args.push(std::mem::take(&mut current_argument));
                            }
                        }
                        State::Inside => {
                            // here we continue taking input from user
                            current_argument.push('\n');
                            print!("quote>");
                            io::stdout().flush().expect("Failed to flush stdout");
                            continue 'outer;
                        }
                    }
                }
                char @ ('\'' | '"') => {
                    match state {
                        State::Outside => {
                            if is_escaping {
                                current_argument.push(char);
                                is_escaping = false;
                                continue;
                            }
                            state = State::Inside;
                            quote = match char {
                                '\'' => Quote::Single,
                                '"' => Quote::Double,
                                _ => Quote::None,
                            }
                        }
                        State::Inside => {
                            if is_escaping && quote == Quote::Double {
                                current_argument.push('"');
                                is_escaping = false;
                            } else {
                                // opposite quote
                                if ((char == '\'') && quote == Quote::Double)
                                    || (char == '"' && quote == Quote::Single)
                                {
                                    current_argument.push(char);
                                } else {
                                    state = State::Outside;
                                }
                            }
                        }
                    }
                }
                char @ (' ' | '\t') => match state {
                    State::Inside => {
                        current_argument.push(char);
                    }
                    State::Outside => {
                        if is_escaping {
                            current_argument.push(char);
                            is_escaping = false;
                        } else if !current_argument.is_empty() {
                            args.push(std::mem::take(&mut current_argument));
                        }
                    }
                },
                '\\' => match state {
                    State::Outside => {
                        if is_escaping {
                            current_argument.push('\\');
                            is_escaping = false;
                        } else {
                            is_escaping = true;
                        }
                    }
                    State::Inside => {
                        if !is_escaping {
                            if quote == Quote::Single {
                                current_argument.push('\\');
                            } else {
                                is_escaping = true;
                            }
                        } else {
                            current_argument.push('\\');
                            is_escaping = false;
                        }
                    }
                },
                any_other_char => {
                    current_argument.push(any_other_char);
                    if is_escaping {
                        is_escaping = false;
                    }
                }
            }
        }

        break;
    }

    args
}

fn main() {
    let prompt = "$ ";
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let commands = command_tokenizer();

        let refs: Vec<&str> = commands.iter().map(String::as_str).collect();

        match refs.as_slice() {
            [] => continue,
            ["exit"] => std::process::exit(0),
            [
                "echo",
                messages @ ..,
                redirect_operator @ (">" | "1>" | "2>"),
                file_path,
            ] => match *redirect_operator {
                "2>" => {
                    write_file(file_path, [""], false).unwrap();
                    println!("{}", messages.join(" "));
                }
                _ => {
                    write_file(file_path, messages.join(" "), false).unwrap();
                }
            },
            ["echo", rest @ ..] => println!("{}", rest.join(" ")),
            ["type", rest @ ..] => type_command(rest),
            ["pwd"] => pwd_command(),
            ["cd", rest @ ..] => cd_command(rest),
            external_command => run_external_command(external_command),
        }
    }
}
