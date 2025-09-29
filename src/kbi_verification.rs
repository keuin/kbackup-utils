use crate::java_objects;
use crate::java_objects::{ObjectCollection2, ObjectElement};
use crate::repo_verification::verify_files;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::thread;

pub fn verify_kbi(kbi_path: String, repo_path: String) {
    let file = File::open(kbi_path).expect("error opening file");
    let mut parser = jaded::Parser::new(file).expect("error decoding file");
    let backup_info: java_objects::SavedIncBackupV1 =
        parser.read_as().expect("error decoding backup info");
    let (send, recv) = crossbeam::channel::bounded(1024);
    thread::spawn(move || {
        traverse_all(
            &PathBuf::from(repo_path),
            &backup_info.object_collection2,
            &send,
        );
    });
    verify_files(0, recv, 0);
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
            &send,
        );
    });
    recv.iter().collect()
}

fn traverse_all(
    root: &PathBuf,
    coll: &ObjectCollection2,
    send: &crossbeam::channel::Sender<(PathBuf, String)>,
) {
    let elements: &HashMap<String, ObjectElement> = coll.elements.as_ref();
    elements.iter().for_each(|(k, v)| {
        let file_name = v.identifier.to_string();
        send.send((root.join(&file_name), file_name))
            .expect("error sending object");
    });
    coll.sub_collections.as_ref().iter().for_each(|(k, v)| {
        traverse_all(&root, v, &send);
    })
}
