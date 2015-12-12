use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;
use std::boxed::Box;
use std::fs::{self, File};
use std::io::{stdout, stdin, Read, Write};
use std::env;
use std::process;
use std::thread;

use self::to_num::ToNum;

pub mod to_num;

macro_rules! readln {
    () => ({
        let mut buffer = String::new();
        match stdin().read_line(&mut buffer) {
            Ok(_) => Some(buffer),
            Err(_) => None
        }
    });
}

/// Structure which represents a Terminal's command.
/// This command structure contains a name, and the code which run the functionnality associated to this one, with zero, one or several argument(s).
/// # Example
/// ```
/// let my_command = Command {
///     name: "my_command",
///     help: "Describe what my_command does followed by a newline showing usage",
///     main: box|args: &Vec<String>| {
///         println!("Say 'hello' to my command! :-D");
///     }
/// }
/// ```
pub struct Command {
    pub name: &'static str,
    pub help: &'static str,
    pub main: Box<Fn(&Vec<String>, &mut Vec<Variable>, &mut Vec<Mode>)>,
}

impl Command {
    /// Return the vector of the commands
    // TODO: Use a more efficient collection instead
    pub fn vec() -> Vec<Self> {
        let mut commands: Vec<Self> = Vec::new();

        commands.push(Command {
            name: "cat",
            help: "To display a file in the output\n    cat <your_file>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                let path = args.get(1).map_or(String::new(), |arg| arg.clone());

                match File::open(&path) {
                    Ok(mut file) => {
                        let mut string = String::new();
                        match file.read_to_string(&mut string) {
                            Ok(_) => println!("{}", string),
                            Err(err) => println!("Failed to read: {}: {}", path, err),
                        }
                    },
                    Err(err) => println!("Failed to open file: {}: {}", path, err)
                }
            }),
        });

        commands.push(Command {
            name: "cd",
            help: "To change the current directory\n    cd <your_destination>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match args.get(1) {
                    Some(path) => {
                        if let Err(err) = env::set_current_dir(&path) {
                            println!("Failed to set current dir to {}: {}", path, err);
                        }
                    }
                    None => println!("No path given"),
                }
            }),
        });

        commands.push(Command {
            name: "echo",
            help: "To display some text in the output\n    echo Hello world!",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                let echo = args.iter()
                               .skip(1)
                               .fold(String::new(), |string, arg| string + " " + arg);
                println!("{}", echo.trim());
            }),
        });

        commands.push(Command {
            name: "else",
            help: "",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {}),
        });

        commands.push(Command {
            name: "exec",
            help: "To execute a binary in the output\n    exec <my_binary>",
            main: Box::new(|args: &Vec<String>, variables: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                if let Some(path) = args.get(1) {
                    let mut command = process::Command::new(path);
                    for i in 2 .. args.len() {
                        if let Some(arg) = args.get(i){
                            command.arg(arg);
                        }
                    }

                    match command.spawn() {
                        Ok(mut child) => {
                            match child.wait() {
                                Ok(status) => {
                                    if let Some(code) = status.code() {
                                        set_var(variables, "?", &format!("{}", code));
                                    } else {
                                        println!("{}: No child exit code", path);
                                    }
                                },
                                Err(err) => println!("{}: Failed to wait: {}", path, err)
                            }
                        },
                        Err(err) => println!("{}: Failed to execute: {}", path, err)
                    }
                }
            }),
        });

        commands.push(Command {
            name: "exit",
            help: "To exit the curent session",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {}),
        });

        commands.push(Command {
            name: "fi",
            help: "",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {}),
        });

        commands.push(Command {
            name: "free",
            help: "Show memory information\n    free",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match File::open("memory:") {
                    Ok(mut file) => {
                        let mut string = String::new();
                        match file.read_to_string(&mut string) {
                            Ok(_) => println!("{}", string),
                            Err(err) => println!("Failed to read: memory: {}", err),
                        }
                    }
                    Err(err) => println!("Failed to open file: memory: {}", err)
                }
            }),
        });

        commands.push(Command {
            name: "if",
            help: "",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {}),
        });

        commands.push(Command {
            name: "ls",
            help: "To list the content of the current directory\n    ls",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                let path = args.get(1).map_or(".".to_string(), |arg| arg.clone());

                match fs::read_dir(&path) {
                    Ok(dir) => {
                        for entry_result in dir {
                            match entry_result {
                                Ok(entry) => {
                                    match entry.path().to_str() {
                                        Some(path_str) => println!("{}", path_str),
                                        None => println!("?")
                                    }
                                },
                                Err(err) => println!("Failed to read entry: {}", err)
                            }
                        }
                    },
                    Err(err) => println!("Failed to open directory: {}: {}", path, err)
                }
            }),
        });

        commands.push(Command {
            name: "mkdir",
            help: "To create a directory in the current directory\n    mkdir <my_new_directory>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match args.get(1) {
                    Some(dir_name) => if let Err(err) = fs::create_dir(dir_name) {
                        println!("Failed to create: {}: {}", dir_name, err);
                    },
                    None => println!("No name provided"),
                }
            }),
        });

        commands.push(Command {
            name: "ps",
            help: "Show process list\n    ps",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match File::open("context:") {
                    Ok(mut file) => {
                        let mut string = String::new();
                        match file.read_to_string(&mut string) {
                            Ok(_) => println!("{}", string),
                            Err(err) => println!("Failed to read: context: {}", err),
                        }
                    }
                    Err(err) => println!("Failed to open file: context: {}", err)
                }
            }),
        });

        commands.push(Command {
            name: "pwd",
            help: "To output the path of the current directory\n    pwd",
            main: Box::new(|_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match env::current_dir() {
                    Ok(path) => match path.to_str() {
                        Some(path_str) => println!("{}", path_str),
                        None => println!("?")
                    },
                    Err(err) => println!("Failed to get current dir: {}", err)
                }
            }),
        });

        commands.push(Command {
            name: "read",
            help: "To read some variables\n    read <my_variable>",
            main: Box::new(|args: &Vec<String>, variables: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                for i in 1..args.len() {
                    if let Some(arg_original) = args.get(i) {
                        let arg = arg_original.trim();
                        print!("{}=", arg);
                        stdout().flush();
                        if let Some(value_original) = readln!() {
                            let value = value_original.trim();
                            set_var(variables, arg, value);
                        }
                    }
                }
            }),
        });

        commands.push(Command {
            name: "rm",
            help: "To remove a file, in the current directory\n    rm <my_file>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match args.get(1) {
                    Some(file_name) => if fs::remove_file(file_name).is_err() {
                        println!("Failed to remove: {}", file_name);
                    },
                    None => println!("No name provided"),
                }
            }),
        });

        commands.push(Command {
            name: "run",
            help: "Run a script\n    run <script>",
            main: Box::new(|args: &Vec<String>, variables: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                let path = "/apps/shell/main.bin";

                let mut command = process::Command::new(path);
                for i in 1 .. args.len() {
                    if let Some(arg) = args.get(i){
                        command.arg(arg);
                    }
                }

                match command.spawn() {
                    Ok(mut child) => {
                        match child.wait() {
                            Ok(status) => {
                                if let Some(code) = status.code() {
                                    set_var(variables, "?", &format!("{}", code));
                                } else {
                                    println!("{}: No child exit code", path);
                                }
                            },
                            Err(err) => println!("{}: Failed to wait: {}", path, err)
                        }
                    },
                    Err(err) => println!("{}: Failed to execute: {}", path, err)
                }
            })
        });

        commands.push(Command {
            name: "sleep",
            help: "Make a sleep in the current session\n    sleep <number_of_seconds>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                let secs = args.get(1).map_or(0, |arg| arg.to_num());
                thread::sleep_ms(secs as u32 * 1000);
            }),
        });

        commands.push(Command {
            name: "send",
            help: "To send data, via an URL\n    send <url> <data>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                if args.len() < 3 {
                    println!("Error: incorrect arguments");
                    println!("Usage: send <url> <data>");
                    return;
                }

                let path = args.get(1).map_or(String::new(), |arg| arg.clone());

                match File::open(&path) {
                    Ok(mut file) => {
                        let string: String = args.iter()
                                                 .skip(2)
                                                 .fold(String::new(), |s, arg| s + " " + arg) +
                                             "\r\n\r\n";

                        match file.write(string.trim_left().as_bytes()) {
                            Ok(size) => println!("Wrote {} bytes", size),
                            Err(err) => println!("Failed to write: {}", err),
                        }

                        let mut string = String::new();
                        match file.read_to_string(&mut string) {
                            Ok(_) => println!("{}", string),
                            Err(err) => println!("Failed to read: {}", err),
                        }
                    },
                    Err(err) => println!("Failed to open: {}", err)
                }
            }),
        });

        // Simple command to create a file, in the current directory
        // The file has got the name given as the first argument of the command
        // If the command have no arguments, the command don't create the file
        commands.push(Command {
            name: "touch",
            help: "To create a file, in the current directory\n    touch <my_file>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                match args.get(1) {
                    Some(file_name) => if let Err(err) = File::create(file_name) {
                        println!("Failed to create: {}: {}", file_name, err);
                    },
                    None => println!("No name provided"),
                }
            }),
        });

        commands.push(Command {
            name: "url_hex",
            help: "",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                let path = args.get(1).map_or(String::new(), |arg| arg.clone());

                match File::open(&path) {
                    Ok(mut file) => {
                        let mut vec: Vec<u8> = vec![];
                        match file.read_to_end(&mut vec) {
                            Ok(_) => {
                                let mut line = "HEX:".to_string();
                                for byte in vec.iter() {
                                    line = line + " " + &format!("{:X}", *byte);
                                }
                                println!("{}", line);
                            }
                            Err(err) => println!("Failed to read: {}", err)
                        }
                    }
                    Err(err) => println!("Failed to open: {}", err)
                }
            }),
        });

        commands.push(Command {
            name: "wget",
            help: "To make some requests at a given host, using TCP protocol\n    wget <host> \
                   <request>",
            main: Box::new(|args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                if let Some(host) = args.get(1) {
                    if let Some(req) = args.get(2) {
                        if let Ok(mut con) = File::open(&("tcp://".to_string() + host)) {
                            con.write(("GET ".to_string() + req + " HTTP/1.1").as_bytes());

                            let mut res = vec![];
                            con.read_to_end(&mut res);

                            if let Ok(mut file) = File::open(&req) {
                                file.write(&res);
                            }
                        }
                    } else {
                        println!("No request given");
                    }
                } else {
                    println!("No url given");
                }
            }),
        });

        // TODO: Someone should implement FromIterator for HashMap before
        //       changing the type back to HashMap
        let command_helper: BTreeMap<String, String> = commands
            .iter()
            .map(|c| (c.name.to_string(), c.help.to_string()))
            .collect();

        commands.push(Command {
            name: "man",
            help: "Display a little helper for a given command\n    man ls",
            main: Box::new(move |args: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                if let Some(command) = args.get(1) {
                    if command_helper.contains_key(command) {
                        match command_helper.get(command) {
                            Some(help) => println!("{}", help),
                            None => println!("Command helper not found [run 'help']..."),
                        }
                    } else {
                        println!("Command helper not found [run 'help']...");
                    }
                } else {
                    println!("Please to specify a command!");
                }
            }),
        });

        let command_list = commands.iter().fold(String::new(), |l, c| l + " " + c.name);

        commands.push(Command {
            name: "help",
            help: "Print available commands",
            main: Box::new(move |_: &Vec<String>, _: &mut Vec<Variable>, _: &mut Vec<Mode>| {
                println!("Commands:{}", command_list);
            }),
        });

        commands
    }
}

/// A (env) variable
pub struct Variable {
    pub name: String,
    pub value: String,
}

pub struct Mode {
    value: bool,
}

fn on_command(command_string: &str,
              commands: &Vec<Command>,
              variables: &mut Vec<Variable>,
              modes: &mut Vec<Mode>) {
    // Comment
    if command_string.starts_with('#') {
        return;
    }

    // Show variables
    if command_string == "$" {
        for variable in variables.iter() {
            println!("{}={}", variable.name, variable.value);
        }
        return;
    }

    // Explode into arguments, replace variables
    let mut args: Vec<String> = vec![];
    for arg in command_string.split(' ') {
        if !arg.is_empty() {
            if arg.starts_with('$') {
                let name = arg[1..arg.len()].to_string();
                for variable in variables.iter() {
                    if variable.name == name {
                        args.push(variable.value.clone());
                        break;
                    }
                }
            } else {
                args.push(arg.to_string());
            }
        }
    }

    // Execute commands
    if let Some(cmd) = args.get(0) {
        if cmd == "if" {
            let mut value = false;

            if let Some(left) = args.get(1) {
                if let Some(cmp) = args.get(2) {
                    if let Some(right) = args.get(3) {
                        if cmp == "==" {
                            value = *left == *right;
                        } else if cmp == "!=" {
                            value = *left != *right;
                        } else if cmp == ">" {
                            value = left.to_num_signed() > right.to_num_signed();
                        } else if cmp == ">=" {
                            value = left.to_num_signed() >= right.to_num_signed();
                        } else if cmp == "<" {
                            value = left.to_num_signed() < right.to_num_signed();
                        } else if cmp == "<=" {
                            value = left.to_num_signed() <= right.to_num_signed();
                        } else {
                            println!("Unknown comparison: {}", cmp);
                        }
                    } else {
                        println!("No right hand side");
                    }
                } else {
                    println!("No comparison operator");
                }
            } else {
                println!("No left hand side");
            }

            modes.insert(0, Mode { value: value });
            return;
        }

        if cmd == "else" {
            let mut syntax_error = false;
            match modes.get_mut(0) {
                Some(mode) => mode.value = !mode.value,
                None => syntax_error = true,
            }
            if syntax_error {
                println!("Syntax error: else found with no previous if");
            }
            return;
        }

        if cmd == "fi" {
            let mut syntax_error = false;
            if !modes.is_empty() {
                modes.remove(0);
            } else {
                syntax_error = true;
            }
            if syntax_error {
                println!("Syntax error: fi found with no previous if");
            }
            return;
        }

        for mode in modes.iter() {
            if !mode.value {
                return;
            }
        }

        // Set variables
        if let Some(i) = cmd.find('=') {
            let name = cmd[0..i].trim();
            let mut value = cmd[i + 1..cmd.len()].trim().to_string();

            for i in 1..args.len() {
                if let Some(arg) = args.get(i) {
                    value = value + " " + &arg;
                }
            }

            set_var(variables, name, &value);
            return;
        }

        // Commands
        for command in commands.iter() {
            if &command.name == cmd {
                (*command.main)(&args, variables, modes);
                return;
            }
        }

        println!("Unknown command: '{}'", cmd);
    }
}


pub fn set_var(variables: &mut Vec<Variable>, name: &str, value: &str) {
    if name.is_empty() {
        return;
    }

    if value.is_empty() {
        let mut remove = -1;
        for i in 0..variables.len() {
            match variables.get(i) {
                Some(variable) => if variable.name == name {
                    remove = i as isize;
                    break;
                },
                None => break,
            }
        }

        if remove >= 0 {
            variables.remove(remove as usize);
        }
    } else {
        for variable in variables.iter_mut() {
            if variable.name == name {
                variable.value = value.to_string();
                return;
            }
        }

        variables.push(Variable {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
}

#[no_mangle] pub fn main() {
    let commands = Command::vec();
    let mut variables: Vec<Variable> = vec![];
    let mut modes: Vec<Mode> = vec![];

    for arg in env::args().skip(1) {
        let mut command_list = String::new();
        if let Ok(mut file) = File::open(arg) {
            file.read_to_string(&mut command_list);
        }

        for command in command_list.split('\n') {
            on_command(&command, &commands, &mut variables, &mut modes);
        }

        return;
    }

    loop {
        for mode in modes.iter().rev() {
            if mode.value {
                print!("+ ");
            } else {
                print!("- ");
            }
        }

        let cwd = match env::current_dir() {
            Ok(path) => match path.to_str() {
                Some(path_str) => path_str.to_string(),
                None => "?".to_string()
            },
            Err(_) => "?".to_string()
        };

        print!("user@redox:{}# ", cwd);
        stdout().flush();

        if let Some(command_original) = readln!() {
            let command = command_original.trim();
            if command == "exit" {
                break;
            } else if !command.is_empty() {
                on_command(&command, &commands, &mut variables, &mut modes);
            }
        } else {
            break;
        }
    }
}
