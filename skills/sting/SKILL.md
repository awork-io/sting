---
name: sting
description: TypeScript dependency analyzer. Use when asking about dependencies, what's affected by changes, finding circular dependencies, or tracing dependency chains between entities.
---

## What this skill does

Use Sting to analyze TypeScript/Angular dependency structure in monorepos.

Sting is especially useful for:
- impact analysis from git diffs
- dependency-path tracing between two entities
- circular dependency detection
- dependency-count ranking
- type-filtered graph generation

## When to use this skill

Use this skill when the user asks:
- what changed impact looks like after a branch/commit diff
- which modules/services/components are affected
- why one entity depends on another
- if there are cycles in the dependency graph
- which entities have the most dependencies

## Project assumptions

- Run commands from repository root unless user provides a path.
- Sting scans TypeScript files in `apps/web`, `apps/mobile`, and `libs`.
- Paths in examples use `<path>` for the analyzed project root.

## Command reference

### Entity discovery

- `sting query-all <path>` - List all discovered entities with IDs and dependencies
- `sting query <path> <name>` - Find details for a specific entity
- `sting unused <path>` - List entities that are defined but not imported

### Graph

- `sting graph <path>` - Output full graph JSON (D3-compatible)
- `sting graph <path> --entity-type component,service` - Filter graph by entity types

### Affected analysis

- `sting affected <path> --base <ref>` - Show direct + consumer impact
- `sting affected <path> --base <ref> --transitive` - Include multi-hop consumers
- `sting affected <path> --base <ref> --paths` - Output only affected directories
- `sting affected <path> --base <ref> --tests` - Output related test files
- `sting affected <path> --base <ref> --project web|mobile|libs` - Filter by project type

### Dependency chain

- `sting chain <path> --start <entity> --end <entity>` - Return all matching paths
- `sting chain <path> --start <entity> --end <entity> --shortest` - Return shortest path only
- `sting chain <path> --start <entity> --end <entity> --max-paths <n>` - Cap number of paths
- `sting chain <path> --start <entity> --end <entity> --max-depth <n>` - Limit traversal depth

### Cycles

- `sting cycles <path>` - Detect circular dependencies
- `sting cycles <path> --max-cycles <n>` - Limit number of cycles
- `sting cycles <path> --max-depth <n>` - Limit cycle length

### Ranking

- `sting rank <path> --by deps` - Rank entities by dependency count
- `sting rank <path> --by deps --entity-type component,service` - Restrict ranking to entity types

### Skill installation

- `sting skill install` - Interactive installation of this skill
- `sting skill install --path ~/.claude/skills/sting` - Install to a directory
- `sting skill install --path ~/.claude/skills/sting/SKILL.md` - Install to a specific file path
- `sting skill install --yes` - Non-interactive install to default location

## Recommended workflows

### 1) Fast impact check after code changes

1. Ask for the base ref if not provided (`main` is a common default).
2. Run: `sting affected <path> --base <ref>`
3. If user wants full blast radius, rerun with `--transitive`.
4. If user needs only runnable targets, use `--paths` or `--tests`.

### 2) Explain why two entities are connected

1. Run: `sting chain <path> --start <A> --end <B>`
2. If output is noisy, rerun with `--shortest`.
3. If no path exists, report that clearly and suggest checking names with `query`.

### 3) Debug architecture issues

1. Run: `sting cycles <path>`
2. Summarize each cycle with entity names first, files second.
3. Suggest highest-leverage breakpoints in the cycle.

## Output guidance for AI agents

- Keep results concise and decision-oriented.
- Prefer short summaries over raw dumps.
- Highlight:
  - direct vs transitive impact
  - file and directory hotspots
  - cycle count and severity
  - top-ranked entities by dependency count
- When output is long, provide:
  - a short summary first
  - then only the most relevant examples

## Option details

### `affected`

- `--base <ref>` (required): branch, tag, or commit SHA to compare against
- `--transitive`: include multi-hop consumers
- `--paths`: output unique directories only
- `--tests`: output full test-file paths
- `--project <type>`: one of `web`, `mobile`, `libs`

### `chain`

- `--shortest`: output only shortest path
- `--max-paths <n>`: default `100`
- `--max-depth <n>`: default `10`

### `cycles`

- `--max-cycles <n>`: default `100`
- `--max-depth <n>`: default `10`

### `graph` and `rank`

- `--entity-type`: comma-separated values from:
  `class`, `component`, `service`, `directive`, `pipe`, `enum`,
  `type`, `interface`, `function`, `const`, `worker`

## Common pitfalls

- Missing/incorrect entity name in `chain` or `query` causes "not found" results.
- Wrong `--base` ref can make affected output misleading.
- `--paths` and `--tests` are optimized for automation; use default mode for human-readable reasoning.
