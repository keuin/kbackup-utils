use std::thread;

use crate::dump_kbi::dump_kbi;
use crate::kbi_verification::verify_kbi;
use crate::repo_verification::verify_incremental_store;
use clap::{Parser, Subcommand};
use signal_hook::{consts::SIGPIPE, iterator::Signals, low_level::exit};

mod archive;
mod dump_kbi;
mod java_objects;
mod kbi_verification;
mod repo_verification;

#[derive(Parser)]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "verify checksum of all incremental backup files")]
    VerifyBackupRepo {
        #[arg(help = "path to the incremental backup directory")]
        path: String,
        #[clap(
            long,
            short,
            help = "how many threads to use; if HDD, set it to 1",
            default_value = "0"
        )]
        threads: usize,
    },
    #[command(about = "dump kbi info in JSON format")]
    DumpKbi {
        #[arg(help = "path to the .kbi file")]
        path: String,
        #[clap(long, action)]
        #[arg(help = "pretty print")]
        pretty: bool,
    },
    #[command(about = "verify checksum of all files in the .kbi file")]
    VerifyKbi {
        #[arg(help = "path to the incremental backup directory")]
        repo_path: String,
        #[arg(help = "path to the .kbi file", trailing_var_arg = true)]
        kbi_path: Vec<String>,
    },
    #[command(about = "archive old incremental backups")]
    Archive {
        #[arg(help = "path to the incremental backup directory")]
        kbi_repo: String,
        #[arg(help = "path to the backups folder")]
        backups: String,
        #[arg(help = "path to the archived incremental backup directory")]
        archive_kbi_repo: String,
        #[arg(help = "path to the archived backups folder")]
        archive_backups: String,
        #[arg(help = "maximum live time of files before being archived")]
        ttl: String,
        #[clap(
            long,
            short,
            help = "do not move any file, just print those actions",
            default_value = "false"
        )]
        dry_run: bool,
    },
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut sigpipe = Signals::new([SIGPIPE]).expect("error creating signal handler");
    thread::spawn(move || {
        for _ in sigpipe.forever() {
            exit(0);
        }
    });

    let cli = CliArgs::parse();
    match cli.command {
        Commands::VerifyBackupRepo { path, threads } => {
            verify_incremental_store(path, threads);
        }
        Commands::DumpKbi { path, pretty } => {
            dump_kbi(path, pretty);
        }
        Commands::VerifyKbi {
            kbi_path,
            repo_path,
        } => {
            verify_kbi(kbi_path, repo_path);
        }
        Commands::Archive {
            kbi_repo,
            backups,
            archive_kbi_repo,
            archive_backups,
            ttl,
            dry_run,
        } => {
            archive::archive_backups(
                kbi_repo,
                backups,
                archive_kbi_repo,
                archive_backups,
                ttl,
                dry_run,
            );
        }
    }
}
