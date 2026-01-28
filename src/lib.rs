mod entity;
mod git;
mod graph;
mod parser;
mod scanner;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::rc::Rc;

use anyhow::Result;

use entity::{Entity, EntityType};
use git::{ChangeType, ChangedFile, get_changed_files};
use graph::DependencyGraph;
use parser::Parser;
use scanner::Scanner;

fn is_test_file(path: &str) -> bool {
    path.ends_with(".test.ts") || path.ends_with(".spec.ts")
}

fn find_test_files_in_directories(directories: &HashSet<String>) -> Vec<String> {
    let mut test_files: HashSet<String> = HashSet::new();

    for dir_path in directories {
        let dir = Path::new(dir_path);
        if !dir.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(path_str) = path.to_str() {
                        if is_test_file(path_str) {
                            test_files.insert(path_str.to_string());
                        }
                    }
                }
            }
        }
    }

    let mut sorted: Vec<String> = test_files.into_iter().collect();
    sorted.sort();
    sorted
}

struct ScanResult {
    entities: HashMap<String, Entity>,
}

fn scan_and_parse_files(root_path: &Path, verbose: bool) -> Result<ScanResult> {
    let subdirs = ["apps/web", "apps/mobile", "libs"];
    let mut all_files = Vec::new();

    let scanner = Scanner::new();

    for subdir in subdirs {
        let full_path = root_path.join(subdir);

        if !full_path.exists() {
            if verbose {
                eprintln!(
                    "Warning: Directory {:?} does not exist, skipping...",
                    full_path
                );
            }
            continue;
        }

        if verbose {
            println!("Scanning directory: {:?}", full_path);
        }

        match scanner.scan(&full_path) {
            Ok(mut files) => {
                if verbose {
                    println!("  Found {} TypeScript files", files.len());
                }
                all_files.append(&mut files);
            }
            Err(e) => {
                if verbose {
                    eprintln!("Warning: Could not read directory {:?}: {}", full_path, e);
                }
            }
        }
    }

    if all_files.is_empty() {
        anyhow::bail!("No TypeScript files found in {}", root_path.display());
    }

    let mut entities_map: HashMap<String, Entity> = HashMap::new();

    if verbose {
        println!("Processing {} TypeScript files...\n", all_files.len());
    }

    let parser = Parser::new(root_path);

    for file in &all_files {
        match parser.parse(file) {
            Ok(result) => {
                for import in &result.imports {
                    if let Some(existing) = entities_map.get_mut(&import.id) {
                        existing.used = true;
                    } else {
                        let mut imported_entity = Entity::new(
                            import.name.clone(),
                            EntityType::Unknown,
                            import.path.clone(),
                            Rc::new(Vec::new()),
                        );
                        imported_entity.used = true;
                        entities_map.insert(import.id.clone(), imported_entity);
                    }
                }

                for entity in result.entities {
                    if let Some(existing) = entities_map.get_mut(&entity.id) {
                        existing.entity_type = entity.entity_type;
                        existing.deps = entity.deps;
                    } else {
                        entities_map.insert(entity.id.clone(), entity);
                    }
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("Warning: Could not parse file {}: {}", file, e);
                }
            }
        }
    }

    Ok(ScanResult {
        entities: entities_map,
    })
}

fn print_entity(entity: &Entity, show_id: bool, show_deps: bool) {
    if show_id {
        println!("ID: {}", entity.id);
    }
    println!("Name: {}", entity.name);
    println!("Type: {}", entity.entity_type);
    println!("File: {}", entity.file_path);
    if show_deps {
        println!("Deps: {:?}", entity.deps);
    }
    println!("---");
}

pub fn query_all(root_path: &Path) -> Result<()> {
    let result = scan_and_parse_files(root_path, true)?;

    println!("Found {} entities:\n", result.entities.len());

    let mut sorted_entities: Vec<_> = result.entities.values().collect();
    sorted_entities.sort_by(|a, b| a.id.cmp(&b.id));

    for entity in sorted_entities {
        print_entity(entity, true, true);
    }

    println!("\nTotal entities in map: {}", result.entities.len());

    Ok(())
}

pub fn query(root_path: &Path, query: &str) -> Result<()> {
    let result = scan_and_parse_files(root_path, false)?;

    if let Some(entity) = result.entities.get(query) {
        print_entity(entity, true, true);
    } else {
        println!("Entity not found: {}", query);
    }

    Ok(())
}

pub fn unused(root_path: &Path) -> Result<()> {
    let result = scan_and_parse_files(root_path, true)?;

    let mut unused_entities: Vec<_> = result
        .entities
        .values()
        .filter(|e| !e.used && !matches!(e.entity_type, EntityType::Unknown))
        .collect();

    unused_entities.sort_by(|a, b| a.file_path.cmp(&b.file_path));

    println!("Found {} unused entities:\n", unused_entities.len());

    for entity in &unused_entities {
        print_entity(entity, false, false);
    }

    println!(
        "\nTotal: {} unused out of {} entities",
        unused_entities.len(),
        result.entities.len()
    );

    Ok(())
}

pub fn graph_json(root_path: &Path) -> Result<String> {
    let result = scan_and_parse_files(root_path, false)?;
    let graph = DependencyGraph::from_entities(&result.entities);
    let json = graph.to_json()?;
    Ok(json)
}

pub fn affected(
    root_path: &Path,
    base_ref: &str,
    transitive: bool,
    paths_only: bool,
    tests_only: bool,
) -> Result<()> {
    if !paths_only && !tests_only {
        println!("Analyzing changes between HEAD and '{}'...\n", base_ref);
    }

    let changed_files = get_changed_files(root_path, base_ref)?;

    if changed_files.is_empty() {
        if !paths_only && !tests_only {
            println!("No changes found between HEAD and '{}'.", base_ref);
        }
        return Ok(());
    }

    if !paths_only && !tests_only {
        println!("Changed files ({}):", changed_files.len());
        for cf in &changed_files {
            println!("  [{}] {}", cf.change_type, cf.path);
        }
        println!();
    }

    let result = scan_and_parse_files(root_path, false)?;

    let graph = DependencyGraph::from_entities(&result.entities);

    let changed_paths: HashSet<String> = changed_files.iter().map(|cf| cf.path.clone()).collect();

    let mut direct_affected: Vec<(&Entity, &ChangedFile)> = Vec::new();
    let mut direct_affected_ids: HashSet<String> = HashSet::new();

    for entity in result.entities.values() {
        if changed_paths.contains(&entity.file_path) {
            if let Some(cf) = changed_files.iter().find(|cf| cf.path == entity.file_path) {
                direct_affected.push((entity, cf));
                direct_affected_ids.insert(entity.id.clone());
            }
        }
    }

    direct_affected.sort_by(|a, b| a.0.file_path.cmp(&b.0.file_path));

    let consumer_ids = graph.find_consumers(&direct_affected_ids, transitive);

    let mut consumers: Vec<(&Entity, String)> = Vec::new();
    for consumer_id in &consumer_ids {
        if let Some(entity) = result.entities.get(consumer_id) {
            let consumes: Vec<String> = entity
                .deps
                .iter()
                .filter_map(|dep| {
                    for (affected_entity, _) in &direct_affected {
                        if affected_entity.file_path == dep.path && affected_entity.name == dep.name
                        {
                            return Some(affected_entity.name.clone());
                        }
                    }
                    None
                })
                .collect();

            let reason = if consumes.is_empty() {
                "Transitive dependency".to_string()
            } else {
                format!("Imports: {}", consumes.join(", "))
            };

            consumers.push((entity, reason));
        }
    }

    consumers.sort_by(|a, b| a.0.file_path.cmp(&b.0.file_path));

    if tests_only {
        let mut test_files: HashSet<String> = HashSet::new();

        // Collect directories from directly affected entities
        let mut affected_dirs: HashSet<String> = HashSet::new();
        for (entity, _) in &direct_affected {
            if let Some(parent) = Path::new(&entity.file_path).parent() {
                affected_dirs.insert(parent.to_string_lossy().to_string());
            }
        }

        // Collect directories from consumer entities
        for (entity, _) in &consumers {
            if let Some(parent) = Path::new(&entity.file_path).parent() {
                affected_dirs.insert(parent.to_string_lossy().to_string());
            }
        }

        // Find test files in those directories
        let discovered_tests = find_test_files_in_directories(&affected_dirs);
        for test_path in discovered_tests {
            test_files.insert(test_path);
        }

        // Include test files that were directly changed in the git diff
        for cf in &changed_files {
            if is_test_file(&cf.path) {
                test_files.insert(cf.path.clone());
            }
        }

        // Output sorted test file paths
        let mut sorted_tests: Vec<String> = test_files.into_iter().collect();
        sorted_tests.sort();

        for test_path in sorted_tests {
            println!("{}", test_path);
        }

        return Ok(());
    }

    if paths_only {
        let mut unique_dirs: HashSet<String> = HashSet::new();

        for (entity, _) in &direct_affected {
            if let Some(parent) = Path::new(&entity.file_path).parent() {
                unique_dirs.insert(parent.to_string_lossy().to_string());
            }
        }

        for (entity, _) in &consumers {
            if let Some(parent) = Path::new(&entity.file_path).parent() {
                unique_dirs.insert(parent.to_string_lossy().to_string());
            }
        }

        let mut sorted_dirs: Vec<_> = unique_dirs.into_iter().collect();
        sorted_dirs.sort();

        for dir in sorted_dirs {
            println!("{}", dir);
        }
    } else {
        println!("---");
        println!("Directly affected entities ({}):\n", direct_affected.len());

        for (entity, cf) in &direct_affected {
            print_affected_entity(
                entity,
                &format!("{} file", change_type_to_reason(&cf.change_type)),
            );
        }

        if !consumers.is_empty() {
            println!("Consumer entities ({}):\n", consumers.len());

            for (entity, reason) in &consumers {
                print_affected_entity(entity, reason);
            }
        }

        let total = direct_affected.len() + consumers.len();
        println!(
            "Summary: {} changed files, {} direct, {} consumers, {} total affected",
            changed_files.len(),
            direct_affected.len(),
            consumers.len(),
            total
        );
    }

    Ok(())
}

fn change_type_to_reason(change_type: &ChangeType) -> &'static str {
    match change_type {
        ChangeType::Added => "New",
        ChangeType::Modified => "Modified",
        ChangeType::Deleted => "Deleted",
        ChangeType::Renamed => "Renamed",
    }
}

fn print_affected_entity(entity: &Entity, reason: &str) {
    println!("Name: {}", entity.name);
    println!("Type: {}", entity.entity_type);
    println!("File: {}", entity.file_path);
    println!("Reason: {}", reason);
    println!("---");
}

#[cfg(test)]
mod tests {
    use super::parser::{Parser, strip_comments};
    use std::path::Path;

    #[test]
    fn test_extract_single_named_import() {
        let content = r#"import { Foo } from './foo';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/bar.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Foo");
        assert!(imports[0].path.contains("foo"));
    }

    #[test]
    fn test_extract_multiple_named_imports() {
        let content = r#"import { Foo, Bar, Baz } from './utils';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].name, "Foo");
        assert_eq!(imports[1].name, "Bar");
        assert_eq!(imports[2].name, "Baz");
    }

    #[test]
    fn test_extract_multiline_named_imports() {
        let content = r#"import {
  Foo,
  Bar,
  Baz
} from './utils';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].name, "Foo");
        assert_eq!(imports[1].name, "Bar");
        assert_eq!(imports[2].name, "Baz");
    }

    #[test]
    fn test_extract_aliased_import() {
        let content = r#"import { Foo as F, Bar as B } from './utils';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].name, "Foo");
        assert_eq!(imports[1].name, "Bar");
    }

    #[test]
    fn test_extract_default_import() {
        let content = r#"import Foo from './foo';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/bar.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Foo");
    }

    #[test]
    fn test_extract_awork_alias_import() {
        let content = r#"import { Model } from '@awork/models';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/apps/web/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Model");
        assert!(imports[0].path.contains("libs/shared/src/lib"));
        assert!(imports[0].path.contains("models"));
        assert!(!imports[0].path.contains("@awork"));
    }

    #[test]
    fn test_skip_external_package_imports() {
        let content = r#"import { useState } from 'react';
import { Observable } from 'rxjs';
import { Foo } from './local';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Foo");
    }

    #[test]
    fn test_extract_multiple_import_statements() {
        let content = r#"import { Foo } from './foo';
import { Bar, Baz } from './bar';
import Default from './default';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].name, "Foo");
        assert_eq!(imports[1].name, "Bar");
        assert_eq!(imports[2].name, "Baz");
        assert_eq!(imports[3].name, "Default");
    }

    #[test]
    fn test_extract_relative_parent_path_import() {
        let content = r#"import { Util } from '../utils/helper';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/components/button.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Util");
        assert!(imports[0].path.contains("utils"));
        assert!(imports[0].path.contains("helper"));
    }

    #[test]
    fn test_import_path_gets_ts_extension() {
        let content = r#"import { Foo } from './foo';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/bar.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert!(imports[0].path.ends_with(".ts"));
    }

    #[test]
    fn test_multiline_import_with_trailing_comma() {
        let content = r#"import {
  Foo,
  Bar,
} from './utils';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].name, "Foo");
        assert_eq!(imports[1].name, "Bar");
    }

    #[test]
    fn test_import_info_has_id() {
        let content = r#"import { Foo } from './foo';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/bar.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert!(!imports[0].id.is_empty());
    }

    #[test]
    fn test_strip_single_line_comment() {
        let content = "const a = 1; // this is a comment\nconst b = 2;";
        let result = strip_comments(content);
        assert_eq!(result, "const a = 1; \nconst b = 2;");
    }

    #[test]
    fn test_strip_multiline_comment() {
        let content = "const a = 1; /* this is\na multiline\ncomment */ const b = 2;";
        let result = strip_comments(content);
        assert_eq!(result, "const a = 1;  const b = 2;");
    }

    #[test]
    fn test_strip_full_line_comment() {
        let content = "// full line comment\nconst a = 1;";
        let result = strip_comments(content);
        assert_eq!(result, "\nconst a = 1;");
    }

    #[test]
    fn test_preserve_string_with_comment_like_content() {
        let content = r#"const a = "// not a comment";"#;
        let result = strip_comments(content);
        assert_eq!(result, r#"const a = "// not a comment";"#);
    }

    #[test]
    fn test_preserve_string_with_multiline_comment_like_content() {
        let content = r#"const a = "/* not a comment */";"#;
        let result = strip_comments(content);
        assert_eq!(result, r#"const a = "/* not a comment */";"#);
    }

    #[test]
    fn test_skip_commented_import() {
        let content = r#"// import { Foo } from './foo';
import { Bar } from './bar';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Bar");
    }

    #[test]
    fn test_skip_multiline_commented_import() {
        let content = r#"/* import { Foo } from './foo'; */
import { Bar } from './bar';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Bar");
    }

    #[test]
    fn test_skip_import_inside_multiline_comment() {
        let content = r#"/*
import { Foo } from './foo';
import { Baz } from './baz';
*/
import { Bar } from './bar';"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/index.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "Bar");
    }

    #[test]
    fn test_extract_angular_lazy_loaded_import() {
        let content = r#"const routes: Routes = [
    {
        path: 'auth',
        loadChildren: () => import('./auth/auth.module').then(m => m.AuthModule)
    }
];"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/app-routing.module.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "AuthModule");
        assert!(imports[0].path.contains("auth/auth.module"));
    }

    #[test]
    fn test_extract_multiple_angular_lazy_loaded_imports() {
        let content = r#"const routes: Routes = [
    {
        path: 'auth',
        loadChildren: () => import('./auth/auth.module').then(m => m.AuthModule)
    },
    {
        path: 'dashboard',
        loadChildren: () => import('./dashboard/dashboard.module').then(mod => mod.DashboardModule)
    }
];"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/app-routing.module.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].name, "AuthModule");
        assert_eq!(imports[1].name, "DashboardModule");
    }

    #[test]
    fn test_extract_lazy_import_with_different_param_names() {
        let content = r#"loadChildren: () => import('./users/users.module').then(module => module.UsersModule)"#;
        let root_path = Path::new("/project");
        let file_path = "/project/src/app-routing.module.ts";

        let parser = Parser::new(root_path);
        let imports = parser.extract_imports(content, file_path);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "UsersModule");
    }

    #[test]
    fn test_is_test_file_spec_ts() {
        assert!(super::is_test_file("/path/to/foo.spec.ts"));
        assert!(super::is_test_file("foo.spec.ts"));
    }

    #[test]
    fn test_is_test_file_test_ts() {
        assert!(super::is_test_file("/path/to/foo.test.ts"));
        assert!(super::is_test_file("foo.test.ts"));
    }

    #[test]
    fn test_is_test_file_non_test_files() {
        assert!(!super::is_test_file("/path/to/foo.ts"));
        assert!(!super::is_test_file("/path/to/foo.spec.tsx"));
        assert!(!super::is_test_file("/path/to/foo.test.tsx"));
        assert!(!super::is_test_file("/path/to/spec.ts.bak"));
    }

    #[test]
    fn test_find_test_files_in_directories() {
        use std::collections::HashSet;
        use std::fs::{self, File};

        // Create a temp directory with test files
        let temp_dir = std::env::temp_dir().join("sting_test_find_tests");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Create test files
        File::create(temp_dir.join("foo.spec.ts")).unwrap();
        File::create(temp_dir.join("bar.test.ts")).unwrap();
        File::create(temp_dir.join("baz.ts")).unwrap();

        let mut dirs: HashSet<String> = HashSet::new();
        dirs.insert(temp_dir.to_string_lossy().to_string());

        let result = super::find_test_files_in_directories(&dirs);

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|p| p.ends_with("foo.spec.ts")));
        assert!(result.iter().any(|p| p.ends_with("bar.test.ts")));
        assert!(!result.iter().any(|p| p.ends_with("baz.ts")));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_test_files_in_nonexistent_directory() {
        use std::collections::HashSet;

        let mut dirs: HashSet<String> = HashSet::new();
        dirs.insert("/nonexistent/path/that/does/not/exist".to_string());

        let result = super::find_test_files_in_directories(&dirs);

        assert!(result.is_empty());
    }
}
