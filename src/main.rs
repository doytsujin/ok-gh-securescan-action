use ratelimit::Ratelimiter;
use reqwest;
// use reqwest::header::{HeaderMap, ACCEPT, AUTHORIZATION};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::{Error, Request, Response};
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

    let rate_limiter = Ratelimiter::builder(1, Duration::from_secs(10))
        .max_tokens(10)
        .initial_available(10)
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
    // let url = format!("https://api.github.com/enterprises/{}/repos", enterprise);
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_static("Bearer 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"),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );

    /*
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
    headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer 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"));
    headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
    let response = client.get("https://api.github.com/repos/doytsujin/ok-gh-securescan-action")
        .headers(headers)
        .send()
        .await;
    if response.status().is_success() {
        let body = response.text().await;
        println!("Response body: {}", body);
    } else {
        eprintln!("Request failed with status: {}", response.status());
    }
    Ok(())
    */

    let max_retries = 10; // Set the maximum number of retries
    for attempt in 0..max_retries {
        match rate_limiter.try_wait() {
            Ok(_) => {
                let request_builder = client
                    .get("https://api.github.com/repos/doytsujin/ok-gh-securescan-action")
                    .basic_auth("doytsujin", Some(""))
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
                /*
                let request = request_builder.build();
                // Print method and URL
                println!("Method: {:?}", request.method());
                println!("URL: {:?}", request.url());
                // Print headers
                for (key, value) in request.headers().iter() {
                    println!("Header: {:?}: {:?}", key, value);
                }
                match client.execute(request).await?;
                {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<Value>().await? {
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
                break; // Exit the loop on success
                */
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
                    // You can choose to exit, return an error, etc.
                    return; // Or use `return Err(e)` if the function returns a Result
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
    // Get the current executable path
    let mut exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(_) => return "Error: Unable to determine executable path".to_string(),
    };

    // Append "output" to the path
    exe_path.push("output");

    // Create the "output" directory if it doesn't exist
    if let Err(_) = fs::create_dir_all(&exe_path) {
        return "Error: Unable to create output directory".to_string();
    }

    // Convert the path to a string
    match exe_path.to_str() {
        Some(path_str) => path_str.to_string(),
        None => "Error: Path contains invalid Unicode".to_string(),
    }
}
