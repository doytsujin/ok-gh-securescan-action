use std::env;
use std::fs::write;
use std::process::exit;

fn main() {
    let github_output_path = env::var("GITHUB_OUTPUT").unwrap();

    let args: Vec<String> = env::args().collect();

    // Check for the presence of at least one argument (the command)
    if args.len() < 2 {
        eprintln!("No command provided");
        write(&github_output_path, "error=No command provided").unwrap();
        exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "clean" => clean(),
        "plan" => {
            if args.len() < 3 {
                eprintln!("No parameter provided for 'plan'");
                write(&github_output_path, "error=No parameter provided for 'plan'").unwrap();
                exit(1);
            }
            let param = &args[2];
            plan(param);
        },
        "run" => run(),
        _ => {
            eprintln!("Invalid command: {}", command);
            write(&github_output_path, &format!("error=Invalid command: {}", command)).unwrap();
            exit(1);
        }
    }
}

fn clean() {
    // Implement clean logic here
    println!("Running 'clean'");
}

fn plan(param: &str) {
    // Implement plan logic here, using the provided parameter
    println!("Running 'plan' with parameter: {}", param);
}

fn run() {
    // Implement run logic here
    println!("Running 'run'");
}
