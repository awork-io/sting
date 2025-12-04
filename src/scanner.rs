use std::fs;
use std::path::Path;

use anyhow::Result;

const DEFAULT_SKIP_DIRECTORIES: &[&str] =
    &["mocks", "__mocks__", "mocks_stubs", "tests", "environments", "i18n"];

const DEFAULT_SKIP_FILE_SUFFIXES: &[&str] = &[
    ".spec.ts",
    ".d.ts",
    ".stories.ts",
    "-stub.ts",
    "mocks.ts",
    "mock.ts",
];

pub(crate) struct Scanner {
    skip_directories: Vec<&'static str>,
    skip_file_suffixes: Vec<&'static str>,
}

impl Scanner {
    pub fn new() -> Self {
        Scanner {
            skip_directories: DEFAULT_SKIP_DIRECTORIES.to_vec(),
            skip_file_suffixes: DEFAULT_SKIP_FILE_SUFFIXES.to_vec(),
        }
    }

    pub fn scan(&self, dir: &Path) -> Result<Vec<String>> {
        let mut ts_files = Vec::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        if let Some(name_str) = dir_name.to_str() {
                            if self.should_skip_directory(name_str) {
                                continue;
                            }
                        }
                    }

                    match self.scan(&path) {
                        Ok(mut nested_files) => ts_files.append(&mut nested_files),
                        Err(e) => eprintln!("Warning: Could not read directory {:?}: {}", path, e),
                    }
                } else if path.is_file() {
                    if self.should_skip_file(&path) {
                        continue;
                    }

                    if let Some(extension) = path.extension() {
                        if extension == "ts" || extension == "tsx" {
                            if let Some(path_str) = path.to_str() {
                                ts_files.push(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(ts_files)
    }

    fn should_skip_directory(&self, dir_name: &str) -> bool {
        self.skip_directories.contains(&dir_name)
    }

    fn should_skip_file(&self, path: &Path) -> bool {
        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                return self
                    .skip_file_suffixes
                    .iter()
                    .any(|suffix| name_str.ends_with(suffix));
            }
        }
        false
    }
}
