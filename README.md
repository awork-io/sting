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

```sh
sting query-all <path>     # List all entities
sting query <path> <name>  # Find specific entity
sting unused <path>        # Find unused entities
sting graph <path>         # Output dependency graph as JSON
sting affected <path>      # List affected files (git-based)
```

## Status

Experimental - APIs may change.

### Suported
- Angular app with a NX style monorepo

## License

MIT
