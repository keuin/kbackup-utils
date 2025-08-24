use crate::dump_kbi::dump_kbi;
use crate::kbi_verification::verify_kbi;
use crate::repo_verification::verify_incremental_store;
use clap::{Parser, Subcommand};

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
        #[arg(help = "path to the .kbi file")]
        kbi_path: String,
    },
}

fn main() {
    tracing_subscriber::fmt::init();
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
    }
}
