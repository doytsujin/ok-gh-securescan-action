use std::env;
use std::fs::write;
use std::process::exit;
use reqwest;
use serde_json::Value;
use ratelimit::Ratelimiter;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() {
    let github_output_path = env::var("GITHUB_OUTPUT").unwrap();

    let args: Vec<String> = env::args().collect();
    let rate_limiter = Ratelimiter::builder(10, Duration::from_secs(10))
        .max_tokens(10)
        .build()
        .unwrap();

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
            plan(param, &rate_limiter).await;
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

async fn plan(enterprise: &str, rate_limiter: &Ratelimiter) {
    let url = format!("https://api.github.com/enterprises/{}/repos", enterprise);
    
    // Wait for a token to become available
    rate_limiter.try_wait();

    // Proceed with the request
    match reqwest::get(&url).await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Value>().await {
                    Ok(repos) => {
                        // Process the list of repositories here
                        println!("Repositories: {:?}", repos);
                    }
                    Err(e) => eprintln!("Failed to parse response: {}", e),
                }
            } else {
                eprintln!("Request failed with status: {}", response.status());
            }
        }
        Err(e) => eprintln!("Failed to send request: {}", e),
    }
}

fn run() {
    // Implement run logic here
    println!("Running 'run'");
}
