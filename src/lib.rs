mod entity;
mod parser;
mod scanner;

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::Result;

use entity::{Entity, EntityType};
use parser::Parser;
use scanner::Scanner;

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
                eprintln!("Warning: Directory {:?} does not exist, skipping...", full_path);
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

#[cfg(test)]
mod tests {
    use super::parser::{strip_comments, Parser};
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
}
