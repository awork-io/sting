use std::collections::{HashMap, HashSet, VecDeque};

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

    /// Build a reverse index mapping target_id -> Vec<source_ids>
    /// This allows us to find all entities that depend on a given entity.
    pub fn build_consumer_index(&self) -> HashMap<String, Vec<String>> {
        let mut index: HashMap<String, Vec<String>> = HashMap::new();

        for edge in &self.edges {
            index
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
        }

        index
    }

    /// Find all entities that consume (depend on) the given target IDs.
    /// If transitive is true, performs BFS to find all transitive consumers.
    /// Returns a set of consumer entity IDs (excluding the original target IDs).
    pub fn find_consumers(
        &self,
        target_ids: &HashSet<String>,
        transitive: bool,
    ) -> HashSet<String> {
        let consumer_index = self.build_consumer_index();
        let mut consumers = HashSet::new();

        if transitive {
            // BFS to find all transitive consumers
            let mut visited = target_ids.clone();
            let mut queue: VecDeque<String> = target_ids.iter().cloned().collect();

            while let Some(current) = queue.pop_front() {
                if let Some(deps) = consumer_index.get(&current) {
                    for consumer_id in deps {
                        if !visited.contains(consumer_id) {
                            visited.insert(consumer_id.clone());
                            queue.push_back(consumer_id.clone());
                            consumers.insert(consumer_id.clone());
                        }
                    }
                }
            }
        } else {
            // Single-hop: only direct consumers
            for target_id in target_ids {
                if let Some(deps) = consumer_index.get(target_id) {
                    for consumer_id in deps {
                        if !target_ids.contains(consumer_id) {
                            consumers.insert(consumer_id.clone());
                        }
                    }
                }
            }
        }

        consumers
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

    #[test]
    fn test_build_consumer_index() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create a target entity (being imported)
        let target = create_entity("Helper", EntityType::Function, "/src/helper.ts", vec![]);
        let target_id = target.id.clone();
        entities.insert(target.id.clone(), target);

        // Create two entities that import the target
        let import = ImportInfo::new("Helper".to_string(), "/src/helper.ts".to_string());
        let consumer1 = create_entity("Service1", EntityType::Class, "/src/service1.ts", vec![import.clone()]);
        let consumer1_id = consumer1.id.clone();
        entities.insert(consumer1.id.clone(), consumer1);

        let consumer2 = create_entity("Service2", EntityType::Class, "/src/service2.ts", vec![import]);
        let consumer2_id = consumer2.id.clone();
        entities.insert(consumer2.id.clone(), consumer2);

        let graph = DependencyGraph::from_entities(&entities);
        let consumer_index = graph.build_consumer_index();

        // The target should have two consumers
        let consumers = consumer_index.get(&target_id).unwrap();
        assert_eq!(consumers.len(), 2);
        assert!(consumers.contains(&consumer1_id));
        assert!(consumers.contains(&consumer2_id));
    }

    #[test]
    fn test_find_consumers_single_hop() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // A -> B -> C chain
        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![import_c]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![import_b]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);

        // Single-hop from C should only find B (direct consumer)
        let mut target_ids = HashSet::new();
        target_ids.insert(c_id);

        let consumers = graph.find_consumers(&target_ids, false);
        assert_eq!(consumers.len(), 1);
        assert!(consumers.contains(&b_id));
        assert!(!consumers.contains(&a_id));
    }

    #[test]
    fn test_find_consumers_transitive() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // A -> B -> C chain
        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![import_c]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![import_b]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);

        // Transitive from C should find both B and A
        let mut target_ids = HashSet::new();
        target_ids.insert(c_id);

        let consumers = graph.find_consumers(&target_ids, true);
        assert_eq!(consumers.len(), 2);
        assert!(consumers.contains(&b_id));
        assert!(consumers.contains(&a_id));
    }

    #[test]
    fn test_find_consumers_handles_cycles() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create a cycle: A -> B -> C -> A
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        // Manually update deps to create cycle
        // A imports C, B imports A, C imports B
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());
        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_c]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);
        entities.get_mut(&c_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);

        let graph = DependencyGraph::from_entities(&entities);

        // Transitive from A should find B and C without infinite loop
        let mut target_ids = HashSet::new();
        target_ids.insert(a_id.clone());

        let consumers = graph.find_consumers(&target_ids, true);
        // B imports A, so B is a consumer. C imports B, so C is a consumer too.
        assert_eq!(consumers.len(), 2);
        assert!(consumers.contains(&b_id));
        assert!(consumers.contains(&c_id));
    }
}
