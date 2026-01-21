mod args;

use std::path::Path;

use anyhow::{Context, Result};
use args::{Commands, StingArgs};
use clap::Parser;

fn canonicalize_path(path_str: &str) -> Result<std::path::PathBuf> {
    let path = Path::new(path_str);
    path.canonicalize()
        .with_context(|| format!("Unable to resolve path: {}", path_str))
}

fn main() -> Result<()> {
    let cli = StingArgs::parse();

    match &cli.command {
        Commands::QueryAll(args) => {
            let path = canonicalize_path(&args.path)?;

            sting::query_all(&path)
                .with_context(|| format!("Unable to query in path: {}", path.display()))?
        }
        Commands::Query(args) => {
            let path = canonicalize_path(&args.path)?;

            sting::query(&path, &args.query)
                .with_context(|| format!("Unable to query in path: {}", path.display()))?
        }
        Commands::Unused(args) => {
            let path = canonicalize_path(&args.path)?;

            sting::unused(&path).with_context(|| {
                format!("Unable to find unused entities in path: {}", path.display())
            })?
        }
        Commands::Graph(args) => {
            let path = canonicalize_path(&args.path)?;

            let json = sting::graph_json(&path).with_context(|| {
                format!("Unable to generate graph for path: {}", path.display())
            })?;

            println!("{}", json);
        }
        Commands::Affected(args) => {
            let path = canonicalize_path(&args.path)?;

            sting::affected(&path, &args.base, args.transitive, args.paths).with_context(
                || {
                    format!(
                        "Unable to find affected entities in path: {}",
                        path.display()
                    )
                },
            )?;
        }
    }

    Ok(())
}
