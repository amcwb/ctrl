use indoc::indoc;
use rocket::serde::json::serde_json;
use slack_rust::{
    chat::post_message::{post_message, PostMessageRequest, PostMessageResponse},
    http_client::SlackWebAPIClient,
    socket::socket_mode::SocketMode,
};

async fn respond_text<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
    text: &String,
) -> Result<PostMessageResponse, slack_rust::error::Error> {
    let request = PostMessageRequest::builder(channel_id.clone())
        .text(text.clone())
        .build();

    post_message(&socket_mode.api_client, &request, &socket_mode.bot_token).await
}

pub async fn command_not_found<S: SlackWebAPIClient>(
    socket_mode: &SocketMode<S>,
    channel_id: &String,
) {
    respond_text(
        socket_mode,
        channel_id,
        &"Invalid command. Use `/ctrl help` for a list of commands.".to_string(),
    )
    .await;
}

pub async fn help<S: SlackWebAPIClient>(socket_mode: &SocketMode<S>, channel_id: &String) {
    let response = respond_text(
        socket_mode,
        channel_id,
        &indoc! {"
            ⛑️ Here's a simple help guide for all the commands available.
 
            - /ctrl help: Show this help guide.
            - /ctrl list: List all projects.
            - /ctrl create <project_name>: Create a new project, automatically assigning it to this channel and adding you as a manager.
            - /ctrl add <@user>: Add a user as a manager to this project
            - /ctrl remove <@user>: Remove a user as a manager from this project
            - /ctrl github <repo_name>: Set the GitHub repository for this project (PRs will be automatically merged, assigned, etc.).
            - /ctrl me github <github_username>: Set your GitHub username.
            "}.to_string(),
    )
    .await;

    println!("{:?} {:?}", channel_id, response);
}
