use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
pub enum ProjectType {
    Web,
    Mobile,
    Libs,
}

#[derive(Clone, Debug, ValueEnum, PartialEq)]
pub enum GraphEntityType {
    Class,
    Component,
    Service,
    Directive,
    Pipe,
    Enum,
    Type,
    Interface,
    Function,
    Const,
    Worker,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum RankBy {
    /// Rank by number of dependencies (outgoing edges)
    Deps,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct StingArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Queries different types of entities in a typescript project
    QueryAll(QueryAllArgs),
    /// Queries different types of entities in a typescript project
    Query(QueryArgs),
    /// Lists all unused entities in the typescript project
    Unused(UnusedArgs),
    /// Outputs the dependency graph as JSON (D3.js compatible)
    Graph(GraphArgs),
    /// Lists all entities affected by git changes compared to a base reference
    Affected(AffectedArgs),
    /// Finds the dependency chain between two entities
    Chain(ChainArgs),
    /// Detects circular dependencies in the project
    Cycles(CyclesArgs),
    /// Ranks entities by various metrics
    Rank(RankArgs),
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// Path to the root of the typescript project
    pub path: String,
    /// Query string to filter entities by
    pub query: String,
}

#[derive(Args, Debug)]
pub struct QueryAllArgs {
    /// Path to the root of the typescript project
    pub path: String,
}

#[derive(Args, Debug)]
pub struct UnusedArgs {
    /// Path to the root of the typescript project
    pub path: String,
}

#[derive(Args, Debug)]
pub struct GraphArgs {
    /// Path to the root of the typescript project
    pub path: String,
    /// Filter to specific entity types (comma-separated, e.g. class,interface)
    #[arg(long, value_enum, value_delimiter = ',')]
    pub entity_type: Vec<GraphEntityType>,
}

#[derive(Args, Debug)]
pub struct AffectedArgs {
    /// Path to the root of the typescript project
    pub path: String,
    /// Git reference to compare against (branch, tag, or commit SHA)
    #[arg(long)]
    pub base: String,
    /// Include transitive consumers (multi-hop dependency traversal)
    #[arg(long, default_value = "false")]
    pub transitive: bool,
    /// Output only unique directory paths (without filenames) for use with test runners
    #[arg(long, default_value = "false", conflicts_with = "tests")]
    pub paths: bool,
    /// Output full paths to test files related to affected entities
    #[arg(long, default_value = "false", conflicts_with = "paths")]
    pub tests: bool,
    /// Filter results to a specific project type (web, mobile, or libs)
    #[arg(long, value_enum)]
    pub project: Option<ProjectType>,
}

#[derive(Args, Debug)]
pub struct ChainArgs {
    /// Path to the root of the typescript project
    pub path: String,
    /// Starting entity name to find chain from
    #[arg(long)]
    pub start: String,
    /// Ending entity name to find chain to
    #[arg(long)]
    pub end: String,
    /// Only return the shortest path (default: return all paths)
    #[arg(long, default_value = "false")]
    pub shortest: bool,
    /// Maximum number of paths to return (default: 100)
    #[arg(long, default_value = "100")]
    pub max_paths: usize,
    /// Maximum path depth/length to explore (default: 10)
    #[arg(long, default_value = "10")]
    pub max_depth: usize,
}

#[derive(Args, Debug)]
pub struct CyclesArgs {
    /// Path to the root of the typescript project
    pub path: String,
    /// Maximum number of cycles to report (default: 100)
    #[arg(long, default_value = "100")]
    pub max_cycles: usize,
    /// Maximum cycle length to detect (default: 10)
    #[arg(long, default_value = "10")]
    pub max_depth: usize,
}

#[derive(Args, Debug)]
pub struct RankArgs {
    /// Path to the root of the typescript project
    pub path: String,
    /// What to rank entities by
    #[arg(long, value_enum)]
    pub by: RankBy,
    /// Filter to specific entity types (comma-separated, e.g. class,interface)
    #[arg(long, value_enum, value_delimiter = ',')]
    pub entity_type: Vec<GraphEntityType>,
}
