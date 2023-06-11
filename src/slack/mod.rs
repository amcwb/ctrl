use reqwest::header::CONTENT_TYPE;
use indoc::indoc;

use crate::{Parameters, Response};

// This should ideally borrow `parameters`, but due to the async block it's not possible.

/// Helper function to respond
fn respond(parameters: Parameters, text: &str) {
    let response = Response {
        text: text.to_string(),
        response_type: "in_channel".to_string(),
    };

    rocket::tokio::spawn(async move {
        // Send response to Slack.
        let _ = reqwest::Client::new()
            .post(&parameters.response_url)
            .header(CONTENT_TYPE, "application/json")
            .json(&response)
            .send()
            .await;
    });
}

pub fn command_handler(parameters: Parameters) {
    let opts = parameters.text.split_whitespace().collect::<Vec<&str>>();

    if opts.len() < 1 {
        respond(parameters.clone(), "Invalid command. Use `/ctrl help` for a list of commands.");
        return;
    };

    let (command, args) = &opts.split_at(1);
    let command = command[0];

    match command {
        "help" => {
            respond(parameters.clone(), indoc! {"
            ⛑️ Here's a simple help guide for all the commands available.

            - /ctrl help: Show this help guide.
            - /ctrl list: List all projects.
            - /ctrl create <project_name>: Create a new project, automatically assigning it to this channel and adding you as a manager.
            - /ctrl add <@github_user>: Add a user as a manager to this project
            - /ctrl remove <@github_user>: Remove a user as a manager from this project
            - /ctrl github <repo_name>: Set the GitHub repository for this project (PRs will be automatically merged, assigned, etc.).
            "})
        },
        "list" => {
            let projects = crate::config::read_manifest().projects;
            let mut response = String::from("Here are all the projects:\n");
            
            for (project_name, project) in projects {
                response.push_str(&format!("- `{}`: ", project_name));

                if project.github_repo.is_some() {
                    response.push_str(&format!("GitHub repo: `{}`", project.github_repo.unwrap()));
                } else {
                    response.push_str("No GitHub repo set");
                }

                // Mention channel.
                response.push_str(&format!(" (channel: <#{}>)", project.slack_channel));
                response.push_str("\n");
            }

            respond(parameters.clone(), &response);
        },
        "create" => {
            if args.len() < 1 {
                respond(parameters.clone(), "Invalid command. Use `/ctrl help` for a list of commands.");
                return;
            }

            let project_name = args[0];
            let mut manifest = crate::config::read_manifest();

            if manifest.projects.contains_key(project_name) {
                respond(parameters.clone(), "Project already exists.");
                return;
            }

            manifest.projects.insert(project_name.to_string(), crate::config::Project {
                slack_channel: parameters.channel_id.to_string(),
                project_owners: vec![],
                github_repo: None,
                jira_project: None,
            });

            crate::config::write_manifest(&manifest);

            respond(parameters.clone(), &format!("Project `{}` created.", project_name));
        },
        "add" => {
            if args.len() < 1 {
                respond(parameters.clone(), "Invalid command. Use `/ctrl help` for a list of commands.");
                return;
            }

            let user = args[0];
            let mut manifest = crate::config::read_manifest();

            // Use attribute slack_channel of projects to iterate over projects and find the one with the matching channel ID.
            let unborrowed_manifest = manifest.clone();
            let project = manifest.projects.iter_mut().find(|(_, project)| project.slack_channel == parameters.channel_id.to_string());
            
            // Throw if not found;
            if project.is_none() {
                respond(parameters.clone(), "Project does not exist.");
                return;
            }

            let (project_name, project) = project.unwrap();

            if project.project_owners.contains(&user.to_string()) {
                respond(parameters.clone(), "User is already a project owner.");
                return;
            }

            // Use github in root of manifest to assign, or fail if otherwise.
            project.project_owners.push(user.to_string());

            crate::config::write_manifest(&manifest);

            respond(parameters.clone(), &format!("User `{}` added as a project owner.", user));
        },
        "remove" => {
            if args.len() < 1 {
                respond(parameters.clone(), "Invalid command. Use `/ctrl help` for a list of commands.");
                return;
            }

            let user = args[0];
            let mut manifest = crate::config::read_manifest();

            // Use attribute slack_channel of projects to iterate over projects and find the one with the matching channel ID.
            let unborrowed_manifest = manifest.clone();
            let project = manifest.projects.iter_mut().find(|(_, project)| project.slack_channel == parameters.channel_id.to_string());
            
            // Throw if not found;
            if project.is_none() {
                respond(parameters.clone(), "Project does not exist.");
                return;
            }

            let (project_name, project) = project.unwrap();

            if !project.project_owners.contains(&user.to_string()) {
                respond(parameters.clone(), "User is not a project owner.");
                return;
            }

            // Use github in root of manifest to assign, or fail if otherwise.
            project.project_owners.retain(|x| x != user);

            crate::config::write_manifest(&manifest);

            respond(parameters.clone(), &format!("User `{}` removed as a project owner.", user));
        }
        "github" => {
            if args.len() < 1 {
                respond(parameters.clone(), "Invalid command. Use `/ctrl help` for a list of commands.");
                return;
            }

            let repo_name = args[0];
            let mut manifest = crate::config::read_manifest();

            let project = manifest.projects.iter_mut().find(|(_, project)| project.slack_channel == parameters.channel_id.to_string());

            match project {
                Some((project_name, project)) => {
                    project.github_repo = Some(repo_name.to_string());
                    crate::config::write_manifest(&manifest);
                    
                    // Respond
                    respond(parameters.clone(), &format!("GitHub repo set to `{}`.", repo_name));
                },
                None => {
                    respond(parameters.clone(), "Project does not exist.");
                }
            }
        },
        _ => {
            respond(parameters.clone(), "Invalid command. Use `/ctrl help` for a list of commands.");
        }
    }
}