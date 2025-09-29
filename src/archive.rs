use std::{
    collections::HashMap,
    fs::{self},
    path::Path,
    process::{self, Stdio, exit},
};

use anyhow::anyhow;
use chrono::{Local, TimeZone};
use lazy_static::lazy_static;
use regex::Regex;

use crate::kbi_verification::collect_kbi_objects;

pub fn archive_backups(
    incr_repo: String,
    backups: String,
    archive_incr_repo: String,
    archive_backups: String,
    dur: String,
    dry_run: bool,
) {
    // 1. list all backups, mark them as active
    // 2. mark old backups (.kbi index files, .zip full backups) as inactive, move them to archive directory
    // 3. list all files in incremental repo, mark them as inactive
    // 4. mark all files referenced in active .kbi files as active
    // 5. move all inactive files from incremental repo to archive directory
    let dur = match duration_str::parse(dur) {
        Ok(v) => v,
        Err(why) => {
            tracing::error!("cannot parse duration string: {}", why);
            exit(1);
        }
    };
    let mut all_backups: HashMap<String, bool> = match fs::read_dir(&backups) {
        Ok(v) => v
            .map(|r| match r {
                Ok(entry) => entry,
                Err(why) => {
                    tracing::error!("failed to list backup file: {}", why);
                    exit(1);
                }
            })
            .filter(|entry| match entry.file_type() {
                Ok(v) => v.is_file(),
                Err(why) => {
                    tracing::error!("failed to get file type: {}", why);
                    exit(1);
                }
            })
            .map(|entry| match entry.file_name().into_string() {
                Ok(file_name) => (file_name, true),
                Err(s) => {
                    tracing::error!("cannot decode OsString {:?}", s);
                    exit(1);
                }
            })
            .collect(),
        Err(why) => {
            tracing::error!("failed to list backup files: {}", why);
            exit(1);
        }
    };
    let mut incr_objects: HashMap<String, bool> = match fs::read_dir(&incr_repo) {
        Ok(v) => v
            .map(|r| match r {
                Ok(entry) => entry,
                Err(why) => {
                    tracing::error!("failed to list incr file: {}", why);
                    exit(1);
                }
            })
            .filter(|entry| match entry.file_type() {
                Ok(v) => v.is_file(),
                Err(why) => {
                    tracing::error!("failed to get file type: {}", why);
                    exit(1);
                }
            })
            .map(|entry| match entry.file_name().into_string() {
                Ok(file_name) => (file_name, false),
                Err(s) => {
                    tracing::error!("cannot decode OsString {:?}", s);
                    exit(1);
                }
            })
            .collect(),
        Err(why) => {
            tracing::error!("failed to list incr files: {}", why);
            exit(1);
        }
    };
    let t0 = Local::now() - dur; // items where create_time < t0 is considered inactive
    for (filename, v) in all_backups.iter_mut() {
        let t = match parse_archive_time_from_filename(&filename) {
            Ok(v) => v,
            Err(why) => {
                tracing::error!("{}", why);
                continue;
            }
        };
        *v = t >= t0;
        if *v {
            tracing::debug!("active: {}", &filename);
        } else {
            tracing::debug!("inactive: {}", &filename);
        }
        if *v && filename.ends_with(".kbi") {
            let p = Path::new(&backups)
                .join(filename)
                .into_os_string()
                .into_string()
                .unwrap();
            // active backup, mark all objects as active
            for (_, obj_filename) in collect_kbi_objects(p, incr_repo.clone()) {
                if !incr_objects.contains_key(&obj_filename) {
                    tracing::error!(
                        "missing file used in backup: {}, used in {}",
                        &obj_filename,
                        &filename
                    );
                    exit(1);
                }
                incr_objects.insert(obj_filename, true);
            }
        }
    }

    all_backups
        .iter()
        .filter(|(_, active)| !**active)
        .for_each(|(backup, _)| {
            if dry_run {
                tracing::info!("archived: {}", backup);
                return;
            }
            if let Err(why) = process::Command::new("mv")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .arg(
                    Path::new(&backups)
                        .join(backup)
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                )
                .arg(
                    Path::new(&archive_backups)
                        .join(backup)
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                )
                .output()
            {
                tracing::error!("error moving backup file {}: {}", backup, why);
                exit(1);
            } else {
                tracing::info!("archived: {}", backup);
            }
        });
    incr_objects
        .iter()
        .filter(|(_, active)| !**active)
        .for_each(|(backup, _)| {
            if dry_run {
                tracing::info!("archived: {}", backup);
                return;
            }
            if let Err(why) = process::Command::new("mv")
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .arg(
                    Path::new(&incr_repo)
                        .join(backup)
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                )
                .arg(
                    Path::new(&archive_incr_repo)
                        .join(backup)
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                )
                .output()
            {
                tracing::error!("error moving incremental object file {}: {}", backup, why);
                exit(1);
            } else {
                tracing::info!("archived: {}", backup);
            }
        });
}

lazy_static! {
    static ref filename_re: Regex =
        Regex::new(r"^(kbackup|incremental)-(\d{4}-\d\d-\d\d_\d\d-\d\d-\d\d)_\S+\.(kbi|zip)$").unwrap();
}

fn parse_archive_time_from_filename(file_name: &str) -> anyhow::Result<chrono::DateTime<Local>> {
    let m = match filename_re.captures(file_name) {
        Some(m) => m.get(2).expect("invalid regex for filename"),
        None => {
            return Err(anyhow!(
                "unrecognized pattern of backup filename: {}",
                file_name
            ));
        }
    };
    match chrono::NaiveDateTime::parse_from_str(m.as_str(), "%Y-%m-%d_%H-%M-%S") {
        Ok(v) => match Local.from_local_datetime(&v).single() {
            Some(v) => Ok(v),
            None => Err(anyhow!("ambiguous local time: {}", m.as_str())),
        },
        Err(why) => Err(anyhow!("error parsing date from filename: {}", why)),
    }
}
