use ratelimit::Ratelimiter;
use reqwest;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::fs::write;
use std::process::exit;
use std::time::Duration;
use tokio;
extern crate uuid;
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

// Define the structs according to your JSON structure
#[derive(Serialize, Deserialize)]
struct Repo {
    name: String,
    security_and_analysis: Option<SecurityAnalysis>,
}

#[derive(Serialize, Deserialize)]
struct SecurityAnalysis {
    secret_scanning: Option<SecretScanning>,
}

#[derive(Serialize, Deserialize)]
struct SecretScanning {
    status: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct NewRepo {
    name: String,
    security_status: String,
}

#[tokio::main]
async fn main() {
    let github_output_path =
        env::var("GITHUB_OUTPUT").unwrap_or_else(|_| String::from(create_output_dir()));

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
        "run" => {
            if args.len() < 3 {
                eprintln!("No parameter provided for 'run'");
                write(&github_output_path, "error=No parameter provided for 'run'").unwrap();
                exit(1);
            }
            // This value represents the number of tokens X that the rate limiter
            // replenishes per some unit of time.
            let tokens_per_unit = env::var("GHSS_TOKENS_PER_UNIT")
                .unwrap_or_else(|_| String::from("1"))
                .parse::<u64>()
                .unwrap_or(0);

            // This value specifies a unit of time as Y seconds. This part suggests
            // that the rate limiter will add X token every Y seconds.
            let unit_of_time = env::var("GHSS_UNIT_OF_TIME")
                .unwrap_or_else(|_| String::from("10"))
                .parse::<u64>()
                .unwrap_or(0);

            // This value sets the maximum number of tokens that the rate limiter can hold.
            // It's set to Z. This means that even if the rate limiter doesn't spend any tokens
            // for a while, it can accumulate at most Z tokens.
            let max_tokens = env::var("GHSS_MAX_TOKENS")
                .unwrap_or_else(|_| String::from("10"))
                .parse::<u64>()
                .unwrap_or(0);

            // This value sets the initial number of available tokens when the rate limiter starts.
            // If this value equals to Z than it means the rate limiter starts off with its
            // maximum capacity of tokens.
            let initial_tokens = env::var("GHSS_INITIAL_TOKENS")
                .unwrap_or_else(|_| String::from("10"))
                .parse::<u64>()
                .unwrap_or(0);

            let rate_limiter =
                Ratelimiter::builder(tokens_per_unit, Duration::from_secs(unit_of_time))
                    .max_tokens(max_tokens)
                    .initial_available(initial_tokens)
                    .build()
                    .unwrap();

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
    let github_token = env::var("GHSS_GITHUB_TOKEN").unwrap_or_else(|_| String::from("UNKNOWN"));
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
    let user_agent = env::var("GHSS_USER_AGENT").unwrap_or_else(|_| String::from("curl/7.68.0"));
    headers.insert(USER_AGENT, user_agent.parse().unwrap());

    // Set the maximum number of retries
    let max_retries = env::var("GHSS_MAX_RETRIES")
        .unwrap_or_else(|_| String::from("10"))
        .parse::<u64>()
        .unwrap_or(0);

    for attempt in 0..max_retries {
        match rate_limiter.try_wait() {
            Ok(_) => {
                let request_builder = client
                    .get(&format!(
                        // "https://api.github.com/repos/{}/ok-gh-securescan-test",
                        // "https://api.github.com/repos/{}/ok-gh-securescan-action",
                        // "https://api.github.com/enterprises/{}/repos",
                        "https://api.github.com/orgs/{}/repos",
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
                                    // println!("Response: {:?}", response);
                                    match response.json::<Value>().await {
                                        Ok(repos) => {
                                            // Process the list of repositories here
                                            println!("Repositories: {:?}", repos);
                                            // Deserialize the JSON data into a Vec<Repo>
                                            let foundrepos: Result<Vec<Repo>, serde_json::Error> =
                                                serde_json::from_str(&repos.to_string());
                                            match foundrepos {
                                                Ok(repos) => {
                                                    let mut new_repos = Vec::new(); // Create a vector to hold the new repo data
                                                    for foundrepo in repos {
                                                        let security_status = foundrepo
                                                            .security_and_analysis
                                                            .and_then(|sa| sa.secret_scanning)
                                                            .and_then(|ss| ss.status)
                                                            .unwrap_or_else(|| "N/A".to_string());

                                                        // Create a new instance of NewRepo
                                                        let new_repo = NewRepo {
                                                            name: foundrepo.name.clone(),
                                                            security_status: security_status
                                                                .clone(),
                                                        };
                                                        new_repos.push(new_repo); // Add it to the vector
                                                        println!(
                                                            "Name: {}, Security Status: {}",
                                                            foundrepo.name, security_status
                                                        );
                                                    }
                                                    // Serialize the new_repos vector to a JSON string
                                                    match serde_json::to_string(&new_repos) {
                                                        Ok(json_string) => {
                                                            let filename =
                                                                format!("{}.json", Uuid::new_v4());
                                                            let temp_dir =
                                                                create_results_dir().clone();
                                                            let dir_path = Path::new(&temp_dir);
                                                            let file_path = dir_path.join(filename);
                                                            std::fs::write(file_path, json_string)
                                                                .expect("Unable to write file");
                                                        }
                                                        Err(e) => {
                                                            println!("Failed to serialize new JSON data: {}", e);
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    println!(
                                                        "Failed to deserialize JSON data: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => eprintln!("Failed to parse response: {}", e),
                                    }
                                    break;
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
        Err(e) => return format!("Error: Unable to determine executable path - {}", e),
    };
    if let Some(parent) = exe_path.parent() {
        exe_path = PathBuf::from(parent);
    }
    exe_path.push("output");
    if let Err(e) = fs::create_dir_all(&exe_path) {
        return format!("Error: Unable to create output directory - {}", e);
    }
    exe_path
        .to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Error: Path contains invalid Unicode".to_string())
}

fn create_results_dir() -> String {
    let mut exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(e) => return format!("Error: Unable to determine executable path - {}", e),
    };
    if let Some(parent) = exe_path.parent() {
        exe_path = PathBuf::from(parent);
    }
    exe_path.push("results");
    if let Err(e) = fs::create_dir_all(&exe_path) {
        return format!("Error: Unable to create results directory - {}", e);
    }
    exe_path
        .to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Error: Path contains invalid Unicode".to_string())
}
