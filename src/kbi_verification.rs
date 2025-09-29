use crate::java_objects;
use crate::java_objects::{ObjectCollection2, ObjectElement};
use crate::repo_verification::verify_files;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;
use std::thread;

pub fn verify_kbi<T: IntoIterator<Item = String> + Send>(kbi_paths: T, repo_path: String) {
    let (send, recv) = crossbeam::channel::bounded(1024);
    crossbeam::thread::scope(|s| {
        s.spawn(move |_| {
            let mut verified_files = HashSet::new();
            for kbi_path in kbi_paths {
                let file = File::open(&kbi_path).expect("error opening file");
                let mut parser = match jaded::Parser::new(file) {
                    Ok(v) => v,
                    Err(why) => {
                        tracing::error!("error decoding kbi file {}: {}", &kbi_path, why);
                        continue;
                    }
                };
                let backup_info: java_objects::SavedIncBackupV1 = match parser.read_as() {
                    Ok(v) => v,
                    Err(why) => {
                        tracing::error!(
                            "error decoding backup info from file {}: {}",
                            &kbi_path,
                            why
                        );
                        continue;
                    }
                };
                traverse_all(
                    &PathBuf::from(&repo_path),
                    &backup_info.object_collection2,
                    &mut |pb, s| {
                        if !verified_files.insert(s.clone()) {
                            return;
                        }
                        send.send((pb, s)).expect("error sending object");
                    },
                );
            }
        });
        verify_files(0, recv, 0);
    })
    .unwrap();
}

pub fn collect_kbi_objects(kbi_path: String, repo_path: String) -> Vec<(PathBuf, String)> {
    let file = File::open(kbi_path).expect("error opening file");
    let mut parser = jaded::Parser::new(file).expect("error decoding file");
    let backup_info: java_objects::SavedIncBackupV1 =
        parser.read_as().expect("error decoding backup info");
    let (send, recv) = crossbeam::channel::bounded(1024);
    thread::spawn(move || {
        traverse_all(
            &PathBuf::from(repo_path),
            &backup_info.object_collection2,
            &mut |pb, s| send.send((pb, s)).expect("error sending object"),
        );
    });
    recv.iter().collect()
}

fn traverse_all<T: FnMut(PathBuf, String) -> ()>(
    root: &PathBuf,
    coll: &ObjectCollection2,
    callback: &mut T,
) {
    let elements: &HashMap<String, ObjectElement> = coll.elements.as_ref();
    elements.iter().for_each(|(k, v)| {
        let file_name = v.identifier.to_string();
        callback(root.join(&file_name), file_name);
    });
    coll.sub_collections.as_ref().iter().for_each(|(k, v)| {
        traverse_all(&root, v, callback);
    })
}
