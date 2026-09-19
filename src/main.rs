use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::result::Result::Ok;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
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

fn create_file_if_not_exist<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let p = path.as_ref();

    // Create parent directories if they don't exist
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    // Only create if it doesn't already exist
    if !p.exists() {
        fs::File::create(p)?;
    }

    Ok(())
}

fn write_file<P, C>(path: P, content: C, append: bool) -> io::Result<()>
where
    P: AsRef<Path>,
    C: FileContent,
{
    let path = path.as_ref();

    create_file_if_not_exist(path)?;

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
                let mut append_to_stdout: bool = false;
                let mut write_stderr_to_file: Option<&str> = None;
                let mut append_to_stderr: bool = false;

                cmd.arg0(command);

                match args {
                    [
                        args_and_path @ ..,
                        operator @ (">" | "1>" | ">>" | "1>>" | "2>" | "2>>"),
                        stdout_file_path,
                    ] => {
                        cmd.args(args_and_path);

                        create_file_if_not_exist(*stdout_file_path).unwrap();

                        match *operator {
                            o @ (">" | "1>" | ">>" | "1>>") => {
                                write_stdout_to_file = Some(*stdout_file_path);
                                if o == "1>>" || o == ">>" {
                                    append_to_stdout = true;
                                }
                            }
                            o @ ("2>" | "2>>") => {
                                write_stderr_to_file = Some(*stdout_file_path);

                                if o == "2>>" {
                                    append_to_stderr = true;
                                }
                            }
                            _ => (),
                        }
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
                                    write_file(path, output.stdout.as_slice(), append_to_stdout)
                                        .unwrap()
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
                                    write_file(path, output.stderr.as_slice(), append_to_stderr)
                                        .unwrap()
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

fn command_completation(user_input: &mut String) {
    let commands = ["echo", "exit"];

    for command in commands {
        if command.starts_with(user_input.as_str()) {
            let end_characters = &command[user_input.len()..];
            print!("{} ", end_characters);

            user_input.push_str(end_characters);
            user_input.push(' ');

            io::stdout().flush().unwrap();
            break;
        }
    }
}

/// RAII guard: enables raw mode on creation, disables it on drop.
/// This runs even on panic (stack unwinding) or early return —
/// you just can't forget to clean up.
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        // Drop can't return Result, so just log if it fails —
        // don't panic inside a Drop impl.
        if let Err(e) = disable_raw_mode() {
            eprintln!("Failed to disable raw mode: {e}");
        }
    }
}

enum InputResult {
    Submitted,
    Cancelled,
}

fn raw_input(user_input: &mut String) -> io::Result<InputResult> {
    let _guard = RawModeGuard::new()?; // raw mode active for the rest of this fn
    user_input.clear();

    let mut buffer = [0u8; 1];

    loop {
        let n = io::stdin().read(&mut buffer)?;
        if n == 0 {
            // EOF (e.g. piped input exhausted, terminal detached)
            return Ok(InputResult::Cancelled);
        }

        match buffer[0] {
            3 => return Ok(InputResult::Cancelled),
            9 => command_completation(&mut *user_input),
            13 => {
                user_input.push('\n');
                print!("\r\n");
                io::stdout().flush()?;
                break;
            }
            127 | 8 => {
                if user_input.pop().is_some() {
                    // move cursor back, overwrite with space, move back again
                    print!("\u{8} \u{8}");
                    io::stdout().flush()?;
                }
            }
            b if b.is_ascii() && !b.is_ascii_control() => {
                let c = b as char;
                user_input.push(c);
                print!("{c}");
                io::stdout().flush()?;
            }
            _ => {}
        }
    }

    Ok(InputResult::Submitted)
    // _guard drops here → disable_raw_mode() runs automatically,
    // whether we got here via break, an early `return`, or a `?` propagating an error.
}

fn read_command(user_input: &mut String) -> io::Result<InputResult> {
    if io::stdin().is_terminal() {
        raw_input(user_input)
    } else {
        user_input.clear();
        let bytes_read = io::stdin().read_line(user_input)?;
        if bytes_read == 0 {
            return Ok(InputResult::Cancelled); // EOF
        }
        Ok(InputResult::Submitted)
    }
}

fn command_tokenizer(prompt: &mut &str) -> Vec<String> {
    let mut state: State = State::Outside;
    let mut quote: Quote = Quote::Single;
    // backslash \ is used outside of quotes, it acts as an escape character
    let mut is_escaping: bool = false;

    let mut args: Vec<String> = Vec::new();
    let mut current_argument: String = String::new();

    let mut user_input = String::new();

    'outer: loop {
        match read_command(&mut user_input) {
            Ok(InputResult::Cancelled) => {
                args.clear();
                current_argument.clear();
                print!("\r\n{}", prompt);
                io::stdout().flush().unwrap();
                user_input.clear();
                continue 'outer;
            }
            Ok(InputResult::Submitted) => {}
            Err(e) => {
                eprintln!("input error: {e}");
                break 'outer;
            }
        }

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
    let mut prompt = "$ ";
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let commands = command_tokenizer(&mut prompt);

        let refs: Vec<&str> = commands.iter().map(String::as_str).collect();

        match refs.as_slice() {
            [] => continue,
            ["exit"] => std::process::exit(0),
            [
                "echo",
                messages @ ..,
                redirect_operator @ (">" | "1>" | "2>" | ">>" | "1>>" | "2>>"),
                file_path,
            ] => match *redirect_operator {
                "2>" | "2>>" => {
                    write_file(file_path, [""], false).unwrap();
                    println!("{}", messages.join(" "));
                }
                rest_operator => {
                    let append = match rest_operator {
                        ">>" | "1>>" => true,
                        _ => false,
                    };
                    let formatted_message = format!("{}\n", messages.join(" "));
                    write_file(file_path, formatted_message, append).unwrap();
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
