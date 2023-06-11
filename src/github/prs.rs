use crate::config::get_project_by_github_repo;

pub async fn handle_pull_request(input: ::rocket::serde::json::Value) {
    let action = input["action"].as_str().unwrap();
    let pull_request = input["pull_request"].clone();

    println!("Received GitHub pull request event: {:?}", action);

    match action {
        "reopened" | "opened" => {
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

            // TODO: Clean up this filter
            let reviewers = project
                .project_owners
                .clone()
                .iter()
                .chain(manifest.managers.iter())
                .filter(|f| **f != pull_request["user"]["login"].as_str().unwrap())
                .map(|f| f.to_owned())
                .collect::<Vec<String>>();

            println!(
                "user {}, reviewers {:?}",
                pull_request["user"]["login"].to_string(),
                reviewers.clone()
            );

            issue_handler
                .add_assignees(
                    pull_request["number"].as_u64().unwrap(),
                    &[pull_request["user"]["login"].as_str().unwrap()],
                )
                .await
                .expect("Failed to assign user");

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
                            "🤖 Thanks @{}. Reviews have been requested from the following project managers: {}",
                            pull_request["user"]["login"].as_str().unwrap(),
                            reviewers.clone().into_iter().map(|f| format!("@{}", f)).collect::<Vec<String>>().join(", ")
                        ),
                    )
                    .await
                    .expect("Failed to create comment");
            } else {
                issue_handler
                    .create_comment(
                        pull_request["number"].as_u64().unwrap(),
                        format!(
                            "🤖 Thanks @{}. I was unable to automatically assign reviews for this PR. Please add them manually: {}",
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

            if review["state"].as_str().unwrap() == "SUBMITTED" {
                pr_handler
                    .merge(pull_request["number"].as_u64().unwrap())
                    .message(format!(
                        "Merge branch {} into {}.\n\n🤖 Approved by {} and automatically merged.",
                        pull_request["head"]["ref"].as_str().unwrap(),
                        pull_request["base"]["ref"].as_str().unwrap(),
                        review["user"]["login"].as_str().unwrap()

                    ))
                    .send()
                    .await
                    .expect("Failed to merge PR");
            }
        }
        _ => (),
    }
}
