use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) enum EntityType {
    Unknown,
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

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EntityType::Unknown => write!(f, "unknown"),
            EntityType::Class => write!(f, "class"),
            EntityType::Component => write!(f, "component"),
            EntityType::Service => write!(f, "service"),
            EntityType::Directive => write!(f, "directive"),
            EntityType::Pipe => write!(f, "pipe"),
            EntityType::Enum => write!(f, "enum"),
            EntityType::Type => write!(f, "type"),
            EntityType::Interface => write!(f, "interface"),
            EntityType::Function => write!(f, "function"),
            EntityType::Const => write!(f, "const"),
            EntityType::Worker => write!(f, "worker"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ImportInfo {
    pub id: String,
    pub name: String,
    pub path: String,
}

impl ImportInfo {
    pub fn new(name: String, path: String) -> Self {
        let id = generate_entity_id(&path, &name);
        ImportInfo { id, name, path }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub file_path: String,
    #[serde(skip)]
    pub deps: Rc<Vec<ImportInfo>>,
    pub used: bool,
}

impl Entity {
    pub fn new(
        name: String,
        entity_type: EntityType,
        file_path: String,
        deps: Rc<Vec<ImportInfo>>,
    ) -> Self {
        let id = generate_entity_id(&file_path, &name);
        Entity {
            id,
            name,
            entity_type,
            file_path,
            deps,
            used: false,
        }
    }
}

pub(crate) fn generate_entity_id(file_path: &str, name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    let key = format!("{}:{}", file_path, name);
    key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
