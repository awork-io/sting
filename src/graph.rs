use std::collections::HashMap;

use serde::Serialize;

use crate::entity::Entity;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GraphNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub file: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    #[serde(rename = "links")]
    pub edges: Vec<GraphEdge>,
}

impl DependencyGraph {
    pub fn from_entities(entities: &HashMap<String, Entity>) -> Self {
        // Build lookup index: (file_path, import_name) -> entity_id
        let mut entity_index: HashMap<(String, String), String> = HashMap::new();
        for entity in entities.values() {
            let key = (entity.file_path.clone(), entity.name.clone());
            entity_index.insert(key, entity.id.clone());
        }

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for entity in entities.values() {
            // Create node for this entity
            nodes.push(GraphNode {
                id: entity.id.clone(),
                name: entity.name.clone(),
                entity_type: entity.entity_type.to_string(),
                file: entity.file_path.clone(),
            });

            // Create edges for each resolved dependency
            for import in entity.deps.iter() {
                // Look up the imported entity by (import.path, import.name)
                let lookup_key = (import.path.clone(), import.name.clone());
                if let Some(target_id) = entity_index.get(&lookup_key) {
                    edges.push(GraphEdge {
                        source: entity.id.clone(),
                        target: target_id.clone(),
                    });
                }
            }
        }

        DependencyGraph { nodes, edges }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{EntityType, ImportInfo};
    use std::rc::Rc;

    fn create_entity(
        name: &str,
        entity_type: EntityType,
        file_path: &str,
        deps: Vec<ImportInfo>,
    ) -> Entity {
        Entity::new(
            name.to_string(),
            entity_type,
            file_path.to_string(),
            Rc::new(deps),
        )
    }

    #[test]
    fn test_empty_entities_produces_empty_graph() {
        let entities: HashMap<String, Entity> = HashMap::new();
        let graph = DependencyGraph::from_entities(&entities);

        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_single_entity_creates_node() {
        let mut entities: HashMap<String, Entity> = HashMap::new();
        let entity = create_entity("MyClass", EntityType::Class, "/src/my-class.ts", vec![]);
        entities.insert(entity.id.clone(), entity);

        let graph = DependencyGraph::from_entities(&entities);

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].name, "MyClass");
        assert_eq!(graph.nodes[0].entity_type, "class");
        assert_eq!(graph.nodes[0].file, "/src/my-class.ts");
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_resolved_import_creates_edge() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create target entity (the one being imported)
        let target = create_entity("Helper", EntityType::Function, "/src/helper.ts", vec![]);
        let target_id = target.id.clone();
        entities.insert(target.id.clone(), target);

        // Create source entity that imports the target
        let import = ImportInfo::new("Helper".to_string(), "/src/helper.ts".to_string());
        let source = create_entity("MyClass", EntityType::Class, "/src/my-class.ts", vec![import]);
        let source_id = source.id.clone();
        entities.insert(source.id.clone(), source);

        let graph = DependencyGraph::from_entities(&entities);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].source, source_id);
        assert_eq!(graph.edges[0].target, target_id);
    }

    #[test]
    fn test_unresolved_import_no_edge() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create entity with import that doesn't resolve to any known entity
        let import = ImportInfo::new("ExternalLib".to_string(), "/external/lib.ts".to_string());
        let entity = create_entity("MyClass", EntityType::Class, "/src/my-class.ts", vec![import]);
        entities.insert(entity.id.clone(), entity);

        let graph = DependencyGraph::from_entities(&entities);

        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_multiple_imports_create_multiple_edges() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create two target entities
        let target1 = create_entity("HelperA", EntityType::Function, "/src/helper-a.ts", vec![]);
        let target1_id = target1.id.clone();
        entities.insert(target1.id.clone(), target1);

        let target2 = create_entity("HelperB", EntityType::Function, "/src/helper-b.ts", vec![]);
        let target2_id = target2.id.clone();
        entities.insert(target2.id.clone(), target2);

        // Create source that imports both
        let imports = vec![
            ImportInfo::new("HelperA".to_string(), "/src/helper-a.ts".to_string()),
            ImportInfo::new("HelperB".to_string(), "/src/helper-b.ts".to_string()),
        ];
        let source = create_entity("MyClass", EntityType::Class, "/src/my-class.ts", imports);
        let source_id = source.id.clone();
        entities.insert(source.id.clone(), source);

        let graph = DependencyGraph::from_entities(&entities);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);

        let edge_targets: Vec<&String> = graph.edges.iter().map(|e| &e.target).collect();
        assert!(edge_targets.contains(&&target1_id));
        assert!(edge_targets.contains(&&target2_id));

        for edge in &graph.edges {
            assert_eq!(edge.source, source_id);
        }
    }

    #[test]
    fn test_self_reference_creates_edge() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Entity A imports Entity B from the same file
        let entity_b = create_entity("HelperB", EntityType::Function, "/src/utils.ts", vec![]);
        let entity_b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let import = ImportInfo::new("HelperB".to_string(), "/src/utils.ts".to_string());
        let entity_a = create_entity("HelperA", EntityType::Function, "/src/utils.ts", vec![import]);
        let entity_a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].source, entity_a_id);
        assert_eq!(graph.edges[0].target, entity_b_id);
    }

    #[test]
    fn test_json_serialization() {
        let mut entities: HashMap<String, Entity> = HashMap::new();
        let entity = create_entity("MyClass", EntityType::Class, "/src/my-class.ts", vec![]);
        entities.insert(entity.id.clone(), entity);

        let graph = DependencyGraph::from_entities(&entities);
        let json = graph.to_json().unwrap();

        // Verify JSON contains expected field names
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"links\"")); // renamed from edges
        assert!(json.contains("\"type\"")); // renamed from entity_type
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"file\""));
        assert!(json.contains("\"MyClass\""));
        assert!(json.contains("\"class\""));
    }

    #[test]
    fn test_json_structure_is_valid() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        let target = create_entity("Helper", EntityType::Function, "/src/helper.ts", vec![]);
        entities.insert(target.id.clone(), target);

        let import = ImportInfo::new("Helper".to_string(), "/src/helper.ts".to_string());
        let source = create_entity("MyClass", EntityType::Class, "/src/my-class.ts", vec![import]);
        entities.insert(source.id.clone(), source);

        let graph = DependencyGraph::from_entities(&entities);
        let json = graph.to_json().unwrap();

        // Parse it back to verify structure
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert!(parsed["nodes"].is_array());
        assert!(parsed["links"].is_array());
        assert_eq!(parsed["nodes"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["links"].as_array().unwrap().len(), 1);
    }
}
