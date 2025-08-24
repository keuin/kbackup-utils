use crate::java_objects;
use std::fs::File;
use std::io;

pub fn dump_kbi(path: String, pretty: bool) {
    let file = File::open(path).expect("error opening file");
    let mut parser = jaded::Parser::new(file).expect("error decoding file");
    let backup_info: java_objects::SavedIncBackupV1 =
        parser.read_as().expect("error decoding backup info");
    let f = if pretty {
        serde_json::to_writer_pretty
    } else {
        serde_json::to_writer
    };
    if let Err(why) = f(io::stdout(), &backup_info) {
        if !why.is_io() {
            tracing::error!("error encoding JSON: {}", why);
        }
    }
}
