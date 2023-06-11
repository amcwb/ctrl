#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;
extern crate reqwest;
extern crate serde;
extern crate toml;

use config::read_manifest;
use rocket::form::Form;
use rocket::http::Status;
use rocket::serde::json::Value;
use serde::Serialize;
use slack::command_handler;

mod config;
mod slack;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Response {
    TextResponse {
        text: String,
        response_type: String,
    },
    BlockResponse {
        blocks: Vec<Value>,
        response_type: String,
    },
}

#[derive(FromForm, Clone)]
pub struct Parameters {
    // Payload fields from "Preparing your app to receive Commands" section in documentation:
    // https://api.slack.com/interactivity/slash-commands#app_command_handling.
    pub token: String,
    pub team_id: String,
    pub team_domain: String,
    pub channel_id: String,
    pub channel_name: String,
    pub user_id: String,
    pub user_name: String,
    pub command: String,
    pub text: String,
    pub response_url: String,
    pub trigger_id: String,
    pub api_app_id: String,
}

#[post("/slack", data = "<input>")]
async fn slack_command(input: Form<Parameters>) -> Status {
    // Unwrap inner object.
    let input_inner = input.into_inner();

    command_handler(input_inner);

    // Return data, respond in background
    Status::Accepted
}

#[post("/github", data = "<input>")]
async fn github_command(input: Form<Parameters>) -> Status {
    // Unwrap inner object.
    let input_inner = input.into_inner();

    command_handler(input_inner);

    // Return data, respond in background
    Status::Accepted
}

#[catch(404)]
fn not_found() -> &'static str {
    // Catch all in case someone goes to the URL directly.
    "You are using this tool incorrectly. Please use it through the command line or Slack."
}

#[rocket::main]
async fn main() {
    // Initialise manifest
    read_manifest();

    // Initialise Rocket, mount root route and register 404 catcher.
    let _ = rocket::build()
        .mount("/", routes![github_command, slack_command])
        .register("/", catchers![not_found])
        .launch()
        .await;
}
