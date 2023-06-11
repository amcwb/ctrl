use std::{fs::File, io::{Write, Read}, path::Path, collections::HashMap};

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub slack_channel: String,
    pub github_repo: Option<String>,
    pub project_owners: Vec<String>,
    pub jira_project: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub projects: HashMap<String, Project>,
    pub managers: Vec<String>,
    pub configured_project: String
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest {
            projects: HashMap::new(),
            managers: Vec::new(),
            configured_project: "amcwb/ctrl".to_string(),
        }
    }
}

pub fn write_manifest(manifest: &Manifest) {
    let mut file = File::create("manifest.toml").unwrap();

    let manifest_json = toml::to_string_pretty(&manifest).unwrap();
    file.write_all(manifest_json.as_bytes()).unwrap();
    let _ = file.sync_all();
    drop(file);

    println!("Wrote manifest.json");
    println!("{:?}", manifest);
}

pub fn read_manifest() -> Manifest {
    if !Path::new("manifest.toml").exists() {
        write_manifest(&Default::default());
    }
    
    let mut file = File::open("manifest.toml").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    let manifest: Manifest = toml::from_str(&contents).unwrap_or(
        Default::default()
    );

    drop(file);

    println!("Read manifest.json");
    println!("{:?}", manifest);
    manifest
}