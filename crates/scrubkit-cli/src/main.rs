// File: crates/scrubkit-cli/src/main.rs

use anyhow::{Context, Result};
use clap::Parser;
use scrubkit_core::{Scrubber, scrubber_for_file};
use std::path::PathBuf;

/// A tool to view and remove potentially sensitive metadata from files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// View metadata for a file
    View {
        /// The path to the file
        #[arg(required = true)]
        file_path: PathBuf,
    },
    /// Remove metadata from a file
    Clean {
        /// The path to the file
        #[arg(required = true)]
        file_path: PathBuf,

        /// Overwrite the file in-place
        #[arg(short, long)]
        in_place: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::View { file_path } => {
            let file_bytes = tokio::fs::read(&file_path)
                .await
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

            // Use the factory function to get the correct scrubber
            let scrubber = scrubber_for_file(file_bytes)?;
            let metadata = scrubber.view_metadata()?;

            if metadata.is_empty() {
                println!("No metadata found in {}.", file_path.display());
            } else {
                println!("Metadata for {}:", file_path.display());
                for entry in metadata {
                    println!("  - {}: {} = {}", entry.category, entry.key, entry.value);
                }
            }
        }

        Commands::Clean {
            file_path,
            in_place,
        } => {
            let file_bytes = tokio::fs::read(&file_path)
                .await
                .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

            // Use the factory function here as well
            let scrubber = scrubber_for_file(file_bytes)?;
            let result = scrubber.scrub()?;

            if result.metadata_removed.is_empty() {
                println!("No metadata found to remove from {}.", file_path.display());
                return Ok(());
            }

            let output_path = if in_place {
                file_path.clone()
            } else {
                let original_name = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");

                let extension = file_path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("bin");
                let new_file_name = format!("{}.clean.{}", original_name, extension);
                file_path.with_file_name(new_file_name)
            };

            tokio::fs::write(&output_path, result.cleaned_file_bytes)
                .await
                .with_context(|| {
                    format!("Failed to write cleaned file to {}", output_path.display())
                })?;

            println!(
                "Successfully removed {} metadata entries.",
                result.metadata_removed.len()
            );
            println!("Cleaned file saved to: {}", output_path.display());
        }
    }

    Ok(())
}
