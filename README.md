# Sting

![Sting](sting.jpg)

> Bilbo's blade that glows blue when enemies are near. Detects problems in a FE project.

A fast CLI for static analysis of Typescript project.

## Why Sting?

- **Fast** - Faster alternative to `sting affected`
- **Static analysis** - Finds issues invisible to linters
- **AI-friendly** - Designed for use with AI tools to reduce context

## Installation

### Cargo
```sh
cargo install sting
```

### Script
Use the `install.sh` script to install in your machine or download
the binaries from the [releases](https://github.com/anfelo/sting/releases) list.

```bash
curl -LSfs https://anfelo.github.io/scripts/install.sh | \
    sh -s -- --git anfelo/sting
```

For more details about this installation script see install.sh -h

## Commands

### query-all

List all entities (components, services, pipes, directives, workers, etc.) in a project.

```sh
sting query-all ./my-project
```

### query

Find a specific entity by name.

```sh
sting query ./my-project UserService
sting query ./my-project "Dashboard"
```

### unused

Find entities that are defined but never imported anywhere.

```sh
sting unused ./my-project
```

### graph

Output the dependency graph as JSON (D3.js compatible format).

```sh
sting graph ./my-project > deps.json
```

### affected

List entities affected by git changes compared to a base reference.

```sh
# Basic usage - compare against main branch
sting affected ./my-project --base main

# Include transitive dependencies (multi-hop)
sting affected ./my-project --base main --transitive

# Output only directory paths (useful for test runners)
sting affected ./my-project --base main --paths

# Output full paths to test files
sting affected ./my-project --base main --tests
```

**Options:**
- `--base` - Git reference to compare against (branch, tag, or commit SHA)
- `--transitive` - Include transitive consumers (multi-hop dependency traversal)
- `--paths` - Output only unique directory paths (without filenames)
- `--tests` - Output full paths to test files related to affected entities

### chain

Find the dependency chain between two entities. Useful for understanding how components are connected.

```sh
# Find all paths from UserService to ApiClient
sting chain ./my-project --start UserService --end ApiClient

# Find only the shortest path
sting chain ./my-project --start UserService --end ApiClient --shortest

# Limit the number of paths returned
sting chain ./my-project --start UserService --end ApiClient --max-paths 10

# Limit the search depth
sting chain ./my-project --start UserService --end ApiClient --max-depth 5
```

**Options:**
- `--start` - Starting entity name
- `--end` - Ending entity name
- `--shortest` - Only return the shortest path (default: return all paths)
- `--max-paths` - Maximum number of paths to return (default: 100)
- `--max-depth` - Maximum path depth/length to explore (default: 10)

### cycles

Detect circular dependencies in the project.

```sh
# Find circular dependencies
sting cycles ./my-project

# Limit the number of cycles reported
sting cycles ./my-project --max-cycles 50

# Limit the maximum cycle length to detect
sting cycles ./my-project --max-depth 5
```

**Options:**
- `--max-cycles` - Maximum number of cycles to report (default: 100)
- `--max-depth` - Maximum cycle length to detect (default: 10)

## Status

Experimental - APIs may change.

### Suported
- Angular app with a NX style monorepo

## License

MIT
