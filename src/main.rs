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

            let entity_type_filters: Vec<String> = args
                .entity_type
                .iter()
                .map(|t| match t {
                    args::GraphEntityType::Class => "class".to_string(),
                    args::GraphEntityType::Component => "component".to_string(),
                    args::GraphEntityType::Service => "service".to_string(),
                    args::GraphEntityType::Directive => "directive".to_string(),
                    args::GraphEntityType::Pipe => "pipe".to_string(),
                    args::GraphEntityType::Enum => "enum".to_string(),
                    args::GraphEntityType::Type => "type".to_string(),
                    args::GraphEntityType::Interface => "interface".to_string(),
                    args::GraphEntityType::Function => "function".to_string(),
                    args::GraphEntityType::Const => "const".to_string(),
                    args::GraphEntityType::Worker => "worker".to_string(),
                })
                .collect();

            let json = sting::graph_json(&path, &entity_type_filters).with_context(|| {
                format!("Unable to generate graph for path: {}", path.display())
            })?;

            println!("{}", json);
        }
        Commands::Affected(args) => {
            let path = canonicalize_path(&args.path)?;

            let project_filter = args.project.as_ref().map(|p| match p {
                args::ProjectType::Web => "apps/web/",
                args::ProjectType::Mobile => "apps/mobile/",
                args::ProjectType::Libs => "libs/",
            });

            sting::affected(
                &path,
                &args.base,
                args.transitive,
                args.paths,
                args.tests,
                project_filter,
            )
            .with_context(|| {
                format!(
                    "Unable to find affected entities in path: {}",
                    path.display()
                )
            })?;
        }
        Commands::Chain(args) => {
            let path = canonicalize_path(&args.path)?;

            sting::chain(
                &path,
                &args.start,
                &args.end,
                args.shortest,
                args.max_paths,
                args.max_depth,
            )
            .with_context(|| format!("Unable to find chain in path: {}", path.display()))?;
        }
        Commands::Cycles(args) => {
            let path = canonicalize_path(&args.path)?;

            sting::cycles(&path, args.max_cycles, args.max_depth)
                .with_context(|| format!("Unable to detect cycles in path: {}", path.display()))?;
        }
        Commands::Rank(args) => {
            let path = canonicalize_path(&args.path)?;

            let entity_type_filters: Vec<String> = args
                .entity_type
                .iter()
                .map(|t| match t {
                    args::GraphEntityType::Class => "class".to_string(),
                    args::GraphEntityType::Component => "component".to_string(),
                    args::GraphEntityType::Service => "service".to_string(),
                    args::GraphEntityType::Directive => "directive".to_string(),
                    args::GraphEntityType::Pipe => "pipe".to_string(),
                    args::GraphEntityType::Enum => "enum".to_string(),
                    args::GraphEntityType::Type => "type".to_string(),
                    args::GraphEntityType::Interface => "interface".to_string(),
                    args::GraphEntityType::Function => "function".to_string(),
                    args::GraphEntityType::Const => "const".to_string(),
                    args::GraphEntityType::Worker => "worker".to_string(),
                })
                .collect();

            match args.by {
                args::RankBy::Deps => {
                    sting::rank_by_deps(&path, &entity_type_filters)
                        .with_context(|| format!("Unable to rank entities in path: {}", path.display()))?;
                }
            }
        }
    }

    Ok(())
}
