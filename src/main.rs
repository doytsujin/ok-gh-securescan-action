use ratelimit::Ratelimiter;
use reqwest;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use serde_json::Value;
use std::env;
use std::fs;
use std::fs::write;
use std::process::exit;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() {
    let github_output_path =
        env::var("GITHUB_OUTPUT").unwrap_or_else(|_| String::from(create_output_dir()));

    let args: Vec<String> = env::args().collect();

    // This value represents the number of tokens that the rate limiter replenishes per some unit of time.
    let tokens_per_unit = env::var("TOKENS_PER_UNIT")
        .unwrap_or_else(|_| String::from("1"))
        .parse::<u64>()
        .unwrap_or(0);

    // This value specifies a unit of time as Y seconds. This part suggests that the rate limiter will add X token every Y seconds.
    let unit_of_time = env::var("UNIT_OF_TIME")
        .unwrap_or_else(|_| String::from("10"))
        .parse::<u64>()
        .unwrap_or(0);

    // This value sets the maximum number of tokens that the rate limiter can hold.
    // It's set to Z.
    // This means that even if the rate limiter doesn't spend any tokens for a while, it can accumulate at most Z tokens.
    let max_tokens = env::var("MAX_TOKENS")
        .unwrap_or_else(|_| String::from("10"))
        .parse::<u64>()
        .unwrap_or(0);

    // This value sets the initial number of available tokens when the rate limiter starts.
    // If this value equals to Z than it means the rate limiter starts off with its maximum capacity of tokens.
    let initial_tokens = env::var("INITIAL_TOKENS")
        .unwrap_or_else(|_| String::from("10"))
        .parse::<u64>()
        .unwrap_or(0);

    let rate_limiter = Ratelimiter::builder(tokens_per_unit, Duration::from_secs(unit_of_time))
        .max_tokens(max_tokens)
        .initial_available(initial_tokens)
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
        "run" => {
            if args.len() < 3 {
                eprintln!("No parameter provided for 'run'");
                write(&github_output_path, "error=No parameter provided for 'run'").unwrap();
                exit(1);
            }
            let param = &args[2];
            run(param, &rate_limiter).await;
        }
        "plan" => plan(),
        _ => {
            eprintln!("Invalid command: {}", command);
            write(
                &github_output_path,
                &format!("error=Invalid command: {}", command),
            )
            .unwrap();
            exit(1);
        }
    }
}

fn clean() {
    // Implement clean logic here
    println!("TODO Running 'clean'");
}

async fn run(enterprise: &str, rate_limiter: &Ratelimiter) {
    let github_token = env::var("GITHUB_TOKEN").unwrap_or_else(|_| String::from("UNKNOWN"));
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", github_token)).unwrap(),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(USER_AGENT, "curl/7.68.0".parse().unwrap());

    let max_retries = 10; // Set the maximum number of retries
    for attempt in 0..max_retries {
        match rate_limiter.try_wait() {
            Ok(_) => {
                let request_builder = client
                    .get(&format!(
                        "https://api.github.com/repos/{}/ok-gh-securescan-action",
                        enterprise
                    ))
                    .headers(headers.clone());
                match request_builder.build() {
                    Ok(request) => {
                        // Print method and URL
                        println!("Method: {:?}", request.method());
                        println!("URL: {:?}", request.url());
                        // Print headers
                        for (key, value) in request.headers().iter() {
                            println!("Header: {:?}: {:?}", key, value);
                        }
                        // Send the request
                        let response = client.execute(request).await;
                        match response {
                            Ok(response) => {
                                if response.status().is_success() {
                                    println!("Response: {:?}", response);
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
                    Err(e) => {
                        eprintln!("Failed to build the request: {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Try and wait on limiter: {:?}", e);
                if attempt < max_retries - 1 {
                    // Log the retry attempt
                    eprintln!("Rate limit exceeded, retrying... (Attempt {})", attempt + 1);
                    // Wait before retrying
                    tokio::time::sleep(Duration::from_secs(1)).await;
                } else {
                    // Max retries exceeded, handle accordingly
                    eprintln!(
                        "Error: Rate limit exceeded after {} attempts: {:?}",
                        max_retries, e
                    );
                    return;
                }
            }
        }
    }
}

fn plan() {
    // Implement run logic here
    println!("TODO Running 'plan'");
}

fn create_output_dir() -> String {
    let mut exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return "Error: Unable to determine executable path".to_string(),
    };

    exe_path.push("output");

    if let Err(_) = fs::create_dir_all(&exe_path) {
        return "Error: Unable to create output directory".to_string();
    }

    match exe_path.to_str() {
        Some(path_str) => path_str.to_string(),
        None => "Error: Path contains invalid Unicode".to_string(),
    }
}
