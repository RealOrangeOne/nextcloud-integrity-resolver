use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs::remove_file;
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize)]
struct IntegrityReportExpectedFile {
    expected: String,
}

#[derive(Deserialize)]
struct IntegrityReport {
    #[serde(default, rename = "EXTRA_FILE")]
    extra_files: HashMap<String, IntegrityReportExpectedFile>,
}

impl IntegrityReport {
    fn get_extra_files(&self) -> Vec<PathBuf> {
        self.extra_files
            .iter()
            .filter(|(_, v)| v.expected.is_empty())
            .map(|(k, _)| PathBuf::from(k))
            .collect()
    }
}

fn get_occ_command() -> Command {
    Command::new(env::var("OCC_PATH").unwrap_or("occ".to_string()))
}

fn get_nextcloud_apps() -> Vec<String> {
    let mut occ_command = get_occ_command();
    let command = occ_command.args(["app:list", "--output", "json"]);

    let output = command.output().expect("Failed to get nextcloud apps");

    let data: Value = serde_json::from_slice(&output.stdout)
        .expect("Received invalid JSON when collecting nextcloud apps");

    let enabled_apps = data["enabled"].as_object().unwrap();
    let disabled_apps = data["disabled"].as_object().unwrap();

    enabled_apps
        .keys()
        .chain(disabled_apps.keys())
        .map(String::clone)
        .collect()
}

fn get_integrity_report<A: AsRef<str>>(app: A) -> Option<IntegrityReport> {
    let mut occ_command = get_occ_command();
    let command = occ_command.args(["integrity:check-app", "--output", "json", app.as_ref()]);

    let output = command.output().expect("Failed to get integrity report");

    if output.stdout == b"[]\n" {
        return None;
    }

    let report: IntegrityReport = serde_json::from_slice(&output.stdout)
        .expect("Received invalid JSON when collecting integrity report");

    // If there are no extra files, pretend there's nothing to do
    if report.extra_files.is_empty() {
        return None;
    }

    Some(report)
}

fn get_app_path<A: AsRef<str>>(app: A) -> PathBuf {
    if let Some(env_app_path) = env::var_os("NEXTCLOUD_APP_PATH") {
        return PathBuf::from(env_app_path).join(app.as_ref());
    }

    let mut occ_command = get_occ_command();
    let command = occ_command.args(["app:getpath", app.as_ref()]);

    let output = command.output().expect("Failed to find app path");

    PathBuf::from(
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string(),
    )
}

fn main() {
    let dry_run = env::var("DRY_RUN").is_ok();

    if dry_run {
        println!("WARNING: Dry-run mode.");
    }

    println!("Listing apps...");
    let apps = get_nextcloud_apps();

    println!("Checking integrity on {} apps...", apps.len());

    let reports: HashMap<String, IntegrityReport> = apps
        .par_iter()
        .map(|app| (app, get_integrity_report(app)))
        .filter_map(|(app, report)| report.map(|r| (String::clone(app), r)))
        .collect();

    println!("Failing apps: {}", reports.len());

    if reports.is_empty() {
        return;
    }

    println!("Collecting files...");
    let unexpected_files: Vec<PathBuf> = reports
        .par_iter()
        .flat_map(|(app, report)| {
            let app_path = get_app_path(app);
            report
                .get_extra_files()
                .into_par_iter()
                .map(move |f| app_path.join(f))
        })
        .collect();

    println!("Found {} unexpected files", unexpected_files.len());

    for file in unexpected_files {
        println!("Removing {}", file.display());
        assert!(file.is_file());
        assert!(!file.is_symlink());
        if !dry_run {
            remove_file(file).expect("Failed to remove file");
        }
    }
}
