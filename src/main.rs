mod commands;
mod config;
mod providers;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "git-ranger")]
#[command(author, version, about = "Manage and synchronize local Git repositories across multiple providers", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new ranger.yaml config in current directory
    Init {
        /// Target directory for config file (defaults to current directory)
        #[arg(short, long, value_name = "DIR")]
        dir: Option<PathBuf>,
    },
    
    /// Synchronize workspace: clone missing repos and fetch updates
    Sync {
        /// Target to sync (group name or repo URL, syncs all if not specified)
        target: Option<String>,
        
        /// Preview what would happen without making changes
        #[arg(short = 'n', long)]
        dry_run: bool,
    },
    
    /// Show status of all configured repos
    Status,
    
    /// List all repos from config with their local paths
    Ls,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { dir } => {
            let target_dir = dir.unwrap_or_else(|| PathBuf::from("."));
            
            match commands::init::init_command(&target_dir) {
                Ok(config_path) => {
                    println!("✓ Initialized git-ranger configuration at {}", config_path.display());
                    println!("\nNext steps:");
                    println!("  1. Edit ranger.yaml with your providers and repositories");
                    println!("  2. Run 'git-ranger sync' to clone and sync everything");
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Err(1)
                }
            }
        }
        Commands::Sync { target, dry_run } => {
            let config_path = PathBuf::from(".").join("ranger.yaml");
            
            let options = commands::sync::SyncOptions {
                config_path,
                target,
                dry_run,
            };
            
            match commands::sync::sync_command(&options) {
                Ok(report) => {
                    if report.errors.is_empty() {
                        Ok(())
                    } else {
                        Err(1)
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    Err(1)
                }
            }
        }
        Commands::Status => {
            eprintln!("Status command not yet implemented");
            Err(1)
        }
        Commands::Ls => {
            eprintln!("Ls command not yet implemented");
            Err(1)
        }
    };

    if let Err(code) = result {
        process::exit(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Just verify the CLI can be instantiated
        let _cli = Cli::parse_from(["git-ranger", "init"]);
    }
}
