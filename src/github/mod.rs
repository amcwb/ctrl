use ::rocket::serde::json::Value;

pub mod prs;
pub mod rocket;

pub fn setup_octocrab() {
    dotenv::dotenv().ok();

    // setup octocrab instance
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    octocrab::initialise(octocrab::Octocrab::builder().personal_token(token).build().unwrap());
}

pub fn github_handler(input: Value, event: crate::github::rocket::GitHubEvent) {
    println!("Received GitHub event: {:?}", event);

    ::rocket::tokio::spawn(async move {
        handle_github_event(input, event).await;
    });
}

pub async fn handle_github_event(input: Value, event: crate::github::rocket::GitHubEvent) {
    match event.0.as_str() {
        "pull_request" => prs::handle_pull_request(input).await,
        _ => (),
    }
}