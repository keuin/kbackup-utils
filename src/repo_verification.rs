use crossbeam::channel::Receiver;
use sha2::{Digest, Sha256};
use std::fs::{File};
use std::path::PathBuf;
use std::process::exit;
use std::{fs, io, thread};

pub fn verify_incremental_store(path: String, mut threads: usize) {
    let n = match fs::read_dir(&path) {
        Ok(p) => p.count(),
        Err(why) => {
            tracing::error!("error reading directory {}", why);
            exit(1);
        }
    };
    let paths = match fs::read_dir(&path) {
        Ok(p) => p,
        Err(why) => {
            tracing::error!("error reading directory {}", why);
            exit(1);
        }
    };
    let (send, recv) = crossbeam::channel::bounded(1024);
    thread::spawn(move || {
        paths
            .filter(|p| p.is_ok())
            .map(|p| p.unwrap())
            .for_each(|p| {
                send.send((
                    p.path(),
                    p.file_name().into_string().expect("invalid file name"),
                ))
                .expect("error writing channel")
            });
    });
    verify_files(threads, recv, n)
}

pub fn verify_files(mut threads: usize, recv: Receiver<(PathBuf, String)>, n: usize) {
    let mut workers = Vec::new();
    // let prog_bar = ProgressBar::new(n as u64);
    if threads <= 0 {
        threads = num_cpus::get();
    }
    for _ in 0..threads {
        // let prog_bar = prog_bar.clone();
        let recv = recv.clone();
        workers.push(thread::spawn(move || {
            recv.iter().for_each(|(path, file_name)| {
                const PREFIX: &str = "S2-";
                if !file_name.starts_with(PREFIX) {
                    tracing::error!("unsupported hash algorithm: {}", file_name);
                    return;
                }
                let expected_hash = &file_name[PREFIX.len()..];
                let actual_hash = match hash_file(path) {
                    Ok(h) => h,
                    Err(why) => {
                        tracing::error!("error hashing file {}: {}", file_name, why);
                        return;
                    }
                };
                if expected_hash != actual_hash {
                    println!(
                        "file hash mismatch: {}, expected: {}, actual: {}",
                        file_name, expected_hash, actual_hash
                    );
                } else {
                    tracing::debug!("checksum OK: {}", file_name);
                }
            })
            // .for_each(|()| prog_bar.inc(1));
        }));
    }

    for w in workers {
        w.join().expect("worker thread panicked");
    }
}

fn hash_file(path: PathBuf) -> anyhow::Result<String> {
    let mut sha256 = Sha256::new();
    let mut file = File::open(path)?;
    io::copy(&mut file, &mut sha256)?;
    Ok(hex::encode_upper(sha256.finalize().to_vec()))
}
