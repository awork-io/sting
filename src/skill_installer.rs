use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const SKILL_CONTENT: &str = include_str!("../skills/sting/SKILL.md");

fn default_skill_install_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home)
        .join(".claude")
        .join("skills")
        .join("sting")
        .join("SKILL.md")
}

fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        return PathBuf::from(env::var("HOME").unwrap_or_else(|_| "~".to_string()));
    }

    if let Some(stripped) = path.strip_prefix("~/") {
        let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
        return Path::new(&home).join(stripped);
    }

    PathBuf::from(path)
}

fn resolve_skill_destination(input: &str) -> PathBuf {
    let path = expand_tilde(input);
    let ends_with_separator = input.ends_with('/') || input.ends_with(std::path::MAIN_SEPARATOR);
    let looks_like_skill_file = path
        .file_name()
        .map(|name| name.to_string_lossy() == "SKILL.md")
        .unwrap_or(false);

    if ends_with_separator || path.is_dir() {
        return path.join("SKILL.md");
    }

    if looks_like_skill_file || path.extension().is_some() {
        return path;
    }

    path.join("SKILL.md")
}

fn prompt_for_skill_path(default_path: &Path) -> Result<PathBuf> {
    print!("Skill install path [{}]: ", default_path.display());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty() {
        Ok(default_path.to_path_buf())
    } else {
        Ok(resolve_skill_destination(trimmed))
    }
}

pub fn install_skill(path_arg: Option<&str>, yes: bool) -> Result<()> {
    let default_path = default_skill_install_path();
    let destination = if let Some(path) = path_arg {
        resolve_skill_destination(path)
    } else if !yes && io::stdin().is_terminal() && io::stdout().is_terminal() {
        prompt_for_skill_path(&default_path)?
    } else {
        default_path
    };

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Unable to create skill directory: {}",
                parent.to_string_lossy()
            )
        })?;
    }

    fs::write(&destination, SKILL_CONTENT).with_context(|| {
        format!(
            "Unable to write skill file to: {}",
            destination.to_string_lossy()
        )
    })?;

    println!("Skill installed at {}", destination.display());
    Ok(())
}
