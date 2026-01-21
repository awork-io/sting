use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct StingArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Queries different types of entities in a nx project
    QueryAll(QueryAllArgs),
    /// Queries different types of entities in a nx project
    Query(QueryArgs),
    /// Lists all unused entities in the nx project
    Unused(UnusedArgs),
    /// Outputs the dependency graph as JSON (D3.js compatible)
    Graph(GraphArgs),
    /// Lists all entities affected by git changes compared to a base reference
    Affected(AffectedArgs),
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// Path to the root of the nx project
    pub path: String,
    /// Query string to filter entities by
    pub query: String,
}

#[derive(Args, Debug)]
pub struct QueryAllArgs {
    /// Path to the root of the nx project
    pub path: String,
}

#[derive(Args, Debug)]
pub struct UnusedArgs {
    /// Path to the root of the nx project
    pub path: String,
}

#[derive(Args, Debug)]
pub struct GraphArgs {
    /// Path to the root of the nx project
    pub path: String,
}

#[derive(Args, Debug)]
pub struct AffectedArgs {
    /// Path to the root of the nx project
    pub path: String,
    /// Git reference to compare against (branch, tag, or commit SHA)
    #[arg(long)]
    pub base: String,
    /// Include transitive consumers (multi-hop dependency traversal)
    #[arg(long, default_value = "false")]
    pub transitive: bool,
    /// Output only unique directory paths (without filenames) for use with test runners
    #[arg(long, default_value = "false")]
    pub paths: bool,
}
