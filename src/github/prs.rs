use itertools::Itertools;

use crate::config::get_project_by_github_repo;

pub async fn handle_pull_request(input: ::rocket::serde::json::Value) {
    let action = input["action"].as_str().unwrap();
    let pull_request = input["pull_request"].clone();

    println!("Received GitHub pull request event: {:?}", action);

    match action {
        "reopened" | "opened" | "ready_for_review" => {
            // Find project by GitHub repo and assign users

            let manifest = crate::config::read_manifest();
            let project = get_project_by_github_repo(
                &manifest,
                pull_request["head"]["repo"]["full_name"].as_str().unwrap(),
            );

            if project.is_none() {
                println!(
                    "No project found for GitHub repo: {}",
                    pull_request["head"]["repo"]["full_name"].as_str().unwrap()
                );
                return;
            }

            let project = project.unwrap();
            let repo = project.github_repo.as_ref().unwrap();
            let details = repo.split("/").collect::<Vec<&str>>();

            let instance = octocrab::instance();
            let issue_handler = instance.issues(details[0].clone(), details[1].clone());
            let pr_handler = instance.pulls(details[0].clone(), details[1].clone());

            let contributors = instance
                .get::<Vec<octocrab::models::Author>, String, ()>(
                    format!(
                        "/repos/{}/{}/contributors",
                        details[0].clone(),
                        details[1].clone()
                    ),
                    None::<&()>,
                )
                .await
                .expect("Failed to get contributors");

            // TODO: Clean up this filter
            let reviewers = project
                .project_owners
                .clone()
                .iter()
                .chain(manifest.managers.iter())
                .filter(|f| **f != pull_request["user"]["login"].as_str().unwrap())
                .filter(|f| contributors.iter().any(|c| c.login == **f))
                .map(|f| f.to_owned())
                .unique()
                .collect::<Vec<String>>();

            issue_handler
                .add_assignees(
                    pull_request["number"].as_u64().unwrap(),
                    &[pull_request["user"]["login"].as_str().unwrap()],
                )
                .await
                .expect("Failed to assign user");

            // Do not request if merging into the wrong branch
            if vec!["master", "main"].contains(&pull_request["base"]["ref"].as_str().unwrap()) {
                issue_handler
                    .create_comment(
                        pull_request["number"].as_u64().unwrap(),
                        format!(
                            "Thanks @{}. This PR is being merged into {}, so I will not request reviews in case this is an error.",
                            pull_request["user"]["login"].as_str().unwrap(),
                            pull_request["base"]["ref"].as_str().unwrap()
                        ),
                    )
                    .await
                    .expect("Failed to create comment");
                return;
            }

            let reviewed = pr_handler
                .request_reviews(
                    pull_request["number"].as_u64().unwrap(),
                    reviewers.clone(),
                    vec![],
                )
                .await;

            if reviewed.is_ok() {
                issue_handler
                    .create_comment(
                        pull_request["number"].as_u64().unwrap(),
                        format!(
                            "Thanks @{}. Reviews have been requested from the following project managers: {} 😊",
                            pull_request["user"]["login"].as_str().unwrap(),
                            reviewers.clone().into_iter().map(|f| format!("@{}", f)).collect::<Vec<String>>().join(", ")
                        ),
                    )
                    .await
                    .expect("Failed to create comment");
            } else {
                println!("{}", reviewed.err().unwrap());
                issue_handler
                    .create_comment(
                        pull_request["number"].as_u64().unwrap(),
                        format!(
                            "Thanks @{}. I was unable to automatically assign reviews for this PR. Please add them manually: {}. 😇",
                            pull_request["user"]["login"].as_str().unwrap(),
                            reviewers.clone().into_iter().map(|f| format!("@{}", f)).collect::<Vec<String>>().join(", ")
                        ),
                    )
                    .await
                    .expect("Failed to create comment");
            }
        }
        _ => (),
    }
}

pub async fn handle_pull_request_review(input: ::rocket::serde::json::Value) {
    let action = input["action"].as_str().unwrap();
    let pull_request = input["pull_request"].clone();
    let review = input["review"].clone();

    println!("Received GitHub pull request review event: {:?}", action);

    match action {
        "submitted" => {
            // Find project by GitHub repo and assign users

            let manifest = crate::config::read_manifest();
            let project = get_project_by_github_repo(
                &manifest,
                pull_request["head"]["repo"]["full_name"].as_str().unwrap(),
            );

            if project.is_none() {
                println!(
                    "No project found for GitHub repo: {}",
                    pull_request["head"]["repo"]["full_name"].as_str().unwrap()
                );
                return;
            }

            let project = project.unwrap();
            let repo = project.github_repo.as_ref().unwrap();
            let details = repo.split("/").collect::<Vec<&str>>();

            let instance = octocrab::instance();
            let issue_handler = instance.issues(details[0].clone(), details[1].clone());
            let pr_handler = instance.pulls(details[0].clone(), details[1].clone());

            println!("Review state: {}", review["state"].as_str().unwrap());

            match review["state"].as_str().unwrap() {
                "approved" => {
                    // Do not merge if merging into the wrong branch
                    if vec!["master", "main"]
                        .contains(&pull_request["base"]["ref"].as_str().unwrap())
                    {
                        issue_handler
                            .create_comment(
                                pull_request["number"].as_u64().unwrap(),
                                format!(
                                    "Thanks @{} for reviewing. This PR is being merged into {}, so I will not merge automatically in case this is an error.",
                                    review["user"]["login"].as_str().unwrap(),
                                    pull_request["base"]["ref"].as_str().unwrap()
                                ),
                            )
                            .await
                            .expect("Failed to create comment");
                        return;
                    }

                    pr_handler
                        .merge(pull_request["number"].as_u64().unwrap())
                        .message(format!(
                            "🤖 Approved by {} and automatically merged on #{}.",
                            review["user"]["login"].as_str().unwrap(),
                            pull_request["number"].as_u64().unwrap()
                        ))
                        .send()
                        .await
                        .expect("Failed to merge PR");

                    issue_handler
                    .create_comment(
                        pull_request["number"].as_u64().unwrap(),
                        format!(
                            "Thanks @{} for reviewing. This will now be automatically merged into {} 😊",
                            review["user"]["login"].as_str().unwrap(),
                            pull_request["base"]["ref"].as_str().unwrap()
                        ),
                    )
                    .await
                    .expect("Failed to create comment");
                }
                "changes_requested" => {
                    issue_handler
                    .create_comment(
                        pull_request["number"].as_u64().unwrap(),
                        format!(
                            "Thanks @{} for reviewing. @{}, make sure these changes are made and a review is requested 😄",
                            review["user"]["login"].as_str().unwrap(),
                            pull_request["user"]["login"].as_str().unwrap()
                        ),
                    )
                    .await
                    .expect("Failed to create comment");
                }
                _ => (),
            }
        }
        _ => (),
    }
}
