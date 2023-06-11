use indoc::indoc;
use reqwest::header::CONTENT_TYPE;
use rocket::serde::json::{serde_json::json, Value};

use crate::{
    config::{get_user_by_slack_id, set_user_github_username},
    Parameters, Response,
};

// This should ideally borrow `parameters`, but due to the async block it's not possible.

/// Helper function to respond
fn respond(parameters: Parameters, text: &str) {
    let response = Response::TextResponse {
        text: text.to_string(),
        response_type: "in_channel".to_string(),
    };

    println!("Sending response: {:?}", response);

    rocket::tokio::spawn(async move {
        // Send response to Slack.
        let res = reqwest::Client::new()
            .post(&parameters.response_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&response)
            .send()
            .await;

        if res.is_err() {
            println!("Error sending response: {:?}", res.err().unwrap());
        } else {
            println!(
                "Response sent successfully: {:?}",
                res.ok().unwrap().text().await.unwrap()
            );
        }
    });
}

fn respond_json<T>(parameters: Parameters, blocks: Vec<T>)
where
    T: Into<Value>,
{
    let response = Response::BlockResponse {
        blocks: blocks.into_iter().map(|x| x.into()).collect(),
        response_type: "in_channel".to_string(),
    };

    println!("Sending response: {:?}", response);

    rocket::tokio::spawn(async move {
        // Send response to Slack.
        let res = reqwest::Client::new()
            .post(&parameters.response_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&response)
            .send()
            .await;

        if res.is_err() {
            println!("Error sending response: {:?}", res.err().unwrap());
        } else {
            println!(
                "Response sent successfully: {:?}",
                res.ok().unwrap().text().await.unwrap()
            );
        }
    });
}

pub fn command_handler(parameters: Parameters) {
    let opts = parameters.text.split_whitespace().collect::<Vec<&str>>();

    if opts.len() < 1 {
        respond(
            parameters.clone(),
            "Invalid command. Use `/ctrl help` for a list of commands.",
        );
        return;
    };

    let (command, args) = &opts.split_at(1);
    let command = command[0];

    match command {
        "help" => respond(
            parameters.clone(),
            indoc! {"
            ⛑️ Here's a simple help guide for all the commands available.

            - /ctrl help: Show this help guide.
            - /ctrl list: List all projects.
            - /ctrl create <project_name>: Create a new project, automatically assigning it to this channel and adding you as a manager.
            - /ctrl add <@user>: Add a user as a manager to this project
            - /ctrl remove <@user>: Remove a user as a manager from this project
            - /ctrl github <repo_name>: Set the GitHub repository for this project (PRs will be automatically merged, assigned, etc.).
            - /ctrl me github <github_username>: Set your GitHub username.
            "},
        ),
        "list" => {
            let projects = crate::config::read_manifest().projects;

            respond_json(
                parameters.clone(),
                vec![json!({
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "Here's a list of all projects:"
                    }
                })]
                .into_iter()
                .chain(
                    projects
                        .into_iter()
                        .map(|(name, project)| match project.github_repo {
                            Some(repo) => json!({
                                "type": "section",
                                "text": {
                                    "type": "mrkdwn",
                                    "text": format!("{} in <#{}>", name, project.slack_channel)
                                },
                                "accessory": {
                                    "type": "button",
                                    "text": {
                                        "type": "plain_text",
                                        "text": "GitHub",
                                        "emoji": true
                                    },
                                    "url": format!("https://github.com/{}", repo),
                                }
                            }),
                            None => json!({
                                "type": "section",
                                "text": {
                                    "type": "mrkdwn",
                                    "text": format!("{} in <#{}>", name, project.slack_channel)
                                },
                            }),
                        })
                        .collect::<Vec<_>>(),
                )
                .collect::<Vec<_>>(),
            );
        }
        "create" => {
            if args.len() < 1 {
                respond(
                    parameters.clone(),
                    "Invalid command. Use `/ctrl help` for a list of commands.",
                );
                return;
            }

            let project_name = args[0];
            let mut manifest = crate::config::read_manifest();

            if manifest.projects.contains_key(project_name) {
                respond(parameters.clone(), "Project already exists.");
                return;
            }

            manifest.projects.insert(
                project_name.to_string(),
                crate::config::Project {
                    slack_channel: parameters.channel_id.to_string(),
                    project_owners: vec![],
                    github_repo: None,
                    jira_project: None,
                },
            );

            crate::config::write_manifest(&manifest);

            respond_json(
                parameters.clone(),
                vec![json!({
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("Project `{}` created.", project_name)
                    }
                })],
            );
        }
        "add" => {
            if args.len() < 1 {
                respond(
                    parameters.clone(),
                    "Invalid command. Use `/ctrl help` for a list of commands.",
                );
                return;
            }

            let user = args[0];
            let mut manifest = crate::config::read_manifest();

            // Use attribute slack_channel of projects to iterate over projects and find the one with the matching channel ID.
            let unborrowed_manifest = manifest.clone();
            let project = manifest
                .projects
                .iter_mut()
                .find(|(_, project)| project.slack_channel == parameters.channel_id.to_string());

            // Throw if not found;
            if project.is_none() {
                respond(parameters.clone(), "Project does not exist.");
                return;
            }

            let (project_name, project) = project.unwrap();

            // Get user from slack
            let slack_id: &str = user
                .trim_start_matches("<@")
                .split("|")
                .collect::<Vec<&str>>()[0];
            println!("User: {}", slack_id);

            let profile = get_user_by_slack_id(&unborrowed_manifest, slack_id);

            if profile.is_none() {
                respond(
                    parameters.clone(),
                    "User has not linked their GitHub account.",
                );
                return;
            }

            let profile = profile.unwrap();

            if project.project_owners.contains(&profile.github_username) {
                respond(parameters.clone(), "User is already a project owner.");
                return;
            }

            // Use github in root of manifest to assign, or fail if otherwise.
            project
                .project_owners
                .push(profile.github_username.to_string());

            crate::config::write_manifest(&manifest);

            respond(
                parameters.clone(),
                &format!("User {} added as a project owner.", user),
            );
        }
        "remove" => {
            if args.len() < 1 {
                respond(
                    parameters.clone(),
                    "Invalid command. Use `/ctrl help` for a list of commands.",
                );
                return;
            }

            let user = args[0];
            let mut manifest = crate::config::read_manifest();

            // Use attribute slack_channel of projects to iterate over projects and find the one with the matching channel ID.
            let unborrowed_manifest = manifest.clone();
            let project = manifest
                .projects
                .iter_mut()
                .find(|(_, project)| project.slack_channel == parameters.channel_id.to_string());

            // Throw if not found;
            if project.is_none() {
                respond(parameters.clone(), "Project does not exist.");
                return;
            }

            let (project_name, project) = project.unwrap();

            // Get user from slack
            let slack_id: &str = user
                .trim_start_matches("<@")
                .split("|")
                .collect::<Vec<&str>>()[0];
            println!("User: {}", slack_id);

            let profile = get_user_by_slack_id(&unborrowed_manifest, slack_id);

            if profile.is_none() {
                respond(
                    parameters.clone(),
                    "User has not linked their GitHub account.",
                );
                return;
            }

            let profile = profile.unwrap();

            if !project.project_owners.contains(&profile.github_username) {
                respond(parameters.clone(), "User is not a project owner.");
                return;
            }

            // Use github in root of manifest to assign, or fail if otherwise.
            project
                .project_owners
                .retain(|username| username != &profile.github_username);

            crate::config::write_manifest(&manifest);

            respond(
                parameters.clone(),
                &format!("User {} removed as a project owner.", user),
            );
        }
        "github" => {
            if args.len() < 1 {
                respond(
                    parameters.clone(),
                    "Invalid command. Use `/ctrl help` for a list of commands.",
                );
                return;
            }

            let repo_name = args[0];
            let mut manifest = crate::config::read_manifest();

            let project = manifest
                .projects
                .iter_mut()
                .find(|(_, project)| project.slack_channel == parameters.channel_id.to_string());

            match project {
                Some((project_name, project)) => {
                    project.github_repo = Some(repo_name.to_string());
                    crate::config::write_manifest(&manifest);

                    // Respond
                    respond(
                        parameters.clone(),
                        &format!("GitHub repo set to `{}`.", repo_name),
                    );
                }
                None => {
                    respond(parameters.clone(), "Project does not exist.");
                }
            }
        }
        "me" => {
            // Get subcommand
            if args.len() < 2 {
                respond(
                    parameters.clone(),
                    "Invalid command. Use `/ctrl help` for a list of commands.",
                );
                return;
            }

            let subcommand = args[0];
            let value = args[1];

            match subcommand {
                "github" => {
                    let mut manifest = crate::config::read_manifest();

                    set_user_github_username(&mut manifest, &parameters.user_id, value);

                    crate::config::write_manifest(&manifest);

                    respond(
                        parameters.clone(),
                        &format!("GitHub username set to `{}`.", value),
                    );
                }
                _ => {
                    respond(
                        parameters.clone(),
                        "Invalid command. Use `/ctrl help` for a list of commands.",
                    );
                }
            }
        }
        _ => {
            respond(
                parameters.clone(),
                "Invalid command. Use `/ctrl help` for a list of commands.",
            );
        }
    }
}
