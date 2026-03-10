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

    /// Find the shortest path from start_id to end_id following dependency edges.
    /// Uses BFS to find the shortest path.
    /// Returns Some(Vec<String>) with entity IDs in path order, or None if no path exists.
    pub fn find_path(&self, start_id: &str, end_id: &str) -> Option<Vec<String>> {
        if start_id == end_id {
            return Some(vec![start_id.to_string()]);
        }

        // Build forward adjacency list: source -> [targets]
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &self.edges {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }

        // BFS to find path
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<Vec<String>> = VecDeque::new();

        visited.insert(start_id.to_string());
        queue.push_back(vec![start_id.to_string()]);

        while let Some(path) = queue.pop_front() {
            let current = path.last().unwrap();

            if let Some(neighbors) = adjacency.get(current) {
                for neighbor in neighbors {
                    if neighbor == end_id {
                        let mut result = path.clone();
                        result.push(neighbor.clone());
                        return Some(result);
                    }

                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push_back(new_path);
                    }
                }
            }
        }

        None
    }

    /// Find all paths from start_id to end_id following dependency edges.
    /// Uses DFS with backtracking to find all possible paths.
    /// Stops early if max_paths is reached or path exceeds max_depth.
    /// Returns a Vec of paths, where each path is a Vec of entity IDs.
    pub fn find_all_paths(
        &self,
        start_id: &str,
        end_id: &str,
        max_paths: usize,
        max_depth: usize,
    ) -> Vec<Vec<String>> {
        let mut all_paths = Vec::new();

        if start_id == end_id {
            return vec![vec![start_id.to_string()]];
        }

        // Build forward adjacency list: source -> [targets]
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &self.edges {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }

        // DFS with backtracking
        let mut visited: HashSet<String> = HashSet::new();
        let mut current_path: Vec<String> = Vec::new();

        self.dfs_all_paths(
            start_id,
            end_id,
            &adjacency,
            &mut visited,
            &mut current_path,
            &mut all_paths,
            max_paths,
            max_depth,
        );

        all_paths
    }

    fn dfs_all_paths(
        &self,
        current: &str,
        end_id: &str,
        adjacency: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        current_path: &mut Vec<String>,
        all_paths: &mut Vec<Vec<String>>,
        max_paths: usize,
        max_depth: usize,
    ) {
        // Early termination if we've found enough paths
        if all_paths.len() >= max_paths {
            return;
        }

        // Early termination if path is too deep
        if current_path.len() >= max_depth {
            return;
        }

        visited.insert(current.to_string());
        current_path.push(current.to_string());

        if current == end_id {
            all_paths.push(current_path.clone());
        } else if let Some(neighbors) = adjacency.get(current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_all_paths(
                        neighbor,
                        end_id,
                        adjacency,
                        visited,
                        current_path,
                        all_paths,
                        max_paths,
                        max_depth,
                    );
                    // Check again after recursive call
                    if all_paths.len() >= max_paths {
                        break;
                    }
                }
            }
        }

        // Backtrack
        current_path.pop();
        visited.remove(current);
    }

    /// Find all circular dependencies (cycles) in the graph.
    /// Uses DFS with node coloring to detect back-edges.
    /// Returns cycles as Vec of entity ID sequences, where the last ID connects back to the first.
    /// Stops early if max_cycles is reached or cycle length exceeds max_depth.
    pub fn find_cycles(&self, max_cycles: usize, max_depth: usize) -> Vec<Vec<String>> {
        // Build forward adjacency list: source -> [targets]
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        let mut all_nodes: HashSet<String> = HashSet::new();

        for edge in &self.edges {
            adjacency
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
            all_nodes.insert(edge.source.clone());
            all_nodes.insert(edge.target.clone());
        }

        // Node states: 0 = white (unvisited), 1 = gray (in stack), 2 = black (done)
        let mut state: HashMap<String, u8> = HashMap::new();
        let mut cycles: Vec<Vec<String>> = Vec::new();
        let mut seen_cycles: HashSet<String> = HashSet::new();

        for start_node in &all_nodes {
            if cycles.len() >= max_cycles {
                break;
            }
            if state.get(start_node).copied().unwrap_or(0) == 0 {
                let mut stack: Vec<String> = Vec::new();
                self.dfs_find_cycles(
                    start_node,
                    &adjacency,
                    &mut state,
                    &mut stack,
                    &mut cycles,
                    &mut seen_cycles,
                    max_cycles,
                    max_depth,
                );
            }
        }

        cycles
    }

    fn dfs_find_cycles(
        &self,
        node: &str,
        adjacency: &HashMap<String, Vec<String>>,
        state: &mut HashMap<String, u8>,
        stack: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
        seen_cycles: &mut HashSet<String>,
        max_cycles: usize,
        max_depth: usize,
    ) {
        if cycles.len() >= max_cycles {
            return;
        }

        if stack.len() >= max_depth {
            return;
        }

        state.insert(node.to_string(), 1); // gray
        stack.push(node.to_string());

        if let Some(neighbors) = adjacency.get(node) {
            for neighbor in neighbors {
                if cycles.len() >= max_cycles {
                    break;
                }

                let neighbor_state = state.get(neighbor).copied().unwrap_or(0);

                if neighbor_state == 1 {
                    // Found a cycle - extract it from stack
                    if let Some(cycle_start) = stack.iter().position(|n| n == neighbor) {
                        let cycle: Vec<String> = stack[cycle_start..].to_vec();

                        // Normalize cycle for deduplication (start from smallest ID)
                        let normalized = self.normalize_cycle(&cycle);
                        let cycle_key = normalized.join(",");

                        if !seen_cycles.contains(&cycle_key) {
                            seen_cycles.insert(cycle_key);
                            cycles.push(cycle);
                        }
                    }
                } else if neighbor_state == 0 {
                    self.dfs_find_cycles(
                        neighbor,
                        adjacency,
                        state,
                        stack,
                        cycles,
                        seen_cycles,
                        max_cycles,
                        max_depth,
                    );
                }
            }
        }

        stack.pop();
        state.insert(node.to_string(), 2); // black
    }

    /// Normalize a cycle by rotating it to start from the lexicographically smallest ID.
    /// This ensures the same cycle found from different starting points is deduplicated.
    fn normalize_cycle(&self, cycle: &[String]) -> Vec<String> {
        if cycle.is_empty() {
            return Vec::new();
        }

        let min_pos = cycle
            .iter()
            .enumerate()
            .min_by_key(|(_, id)| id.as_str())
            .map(|(i, _)| i)
            .unwrap_or(0);

        let mut normalized: Vec<String> = cycle[min_pos..].to_vec();
        normalized.extend_from_slice(&cycle[..min_pos]);
        normalized
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
        let source = create_entity(
            "MyClass",
            EntityType::Class,
            "/src/my-class.ts",
            vec![import],
        );
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
        let entity = create_entity(
            "MyClass",
            EntityType::Class,
            "/src/my-class.ts",
            vec![import],
        );
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
        let entity_a = create_entity(
            "HelperA",
            EntityType::Function,
            "/src/utils.ts",
            vec![import],
        );
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
        let source = create_entity(
            "MyClass",
            EntityType::Class,
            "/src/my-class.ts",
            vec![import],
        );
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
        let consumer1 = create_entity(
            "Service1",
            EntityType::Class,
            "/src/service1.ts",
            vec![import.clone()],
        );
        let consumer1_id = consumer1.id.clone();
        entities.insert(consumer1.id.clone(), consumer1);

        let consumer2 = create_entity(
            "Service2",
            EntityType::Class,
            "/src/service2.ts",
            vec![import],
        );
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

    #[test]
    fn test_find_path_same_entity() {
        let mut entities: HashMap<String, Entity> = HashMap::new();
        let entity = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity.id.clone();
        entities.insert(entity.id.clone(), entity);

        let graph = DependencyGraph::from_entities(&entities);
        let path = graph.find_path(&a_id, &a_id);

        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![a_id]);
    }

    #[test]
    fn test_find_path_direct_dependency() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // A imports B
        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![import_b]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);
        let path = graph.find_path(&a_id, &b_id);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], a_id);
        assert_eq!(path[1], b_id);
    }

    #[test]
    fn test_find_path_transitive() {
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
        let path = graph.find_path(&a_id, &c_id);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], a_id);
        assert_eq!(path[1], b_id);
        assert_eq!(path[2], c_id);
    }

    #[test]
    fn test_find_path_no_connection() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // A and B are not connected
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let graph = DependencyGraph::from_entities(&entities);
        let path = graph.find_path(&a_id, &b_id);

        assert!(path.is_none());
    }

    #[test]
    fn test_find_path_handles_cycles() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create cycle: A -> B -> C -> A, but find path A -> C
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_c]);
        entities.get_mut(&c_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);

        let graph = DependencyGraph::from_entities(&entities);
        let path = graph.find_path(&a_id, &c_id);

        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], a_id);
        assert_eq!(path[1], b_id);
        assert_eq!(path[2], c_id);
    }

    #[test]
    fn test_find_all_paths_same_entity() {
        let mut entities: HashMap<String, Entity> = HashMap::new();
        let entity = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity.id.clone();
        entities.insert(entity.id.clone(), entity);

        let graph = DependencyGraph::from_entities(&entities);
        let paths = graph.find_all_paths(&a_id, &a_id, 100, 100);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], vec![a_id]);
    }

    #[test]
    fn test_find_all_paths_single_path() {
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
        let paths = graph.find_all_paths(&a_id, &c_id, 100, 100);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 3);
        assert_eq!(paths[0][0], a_id);
        assert_eq!(paths[0][1], b_id);
        assert_eq!(paths[0][2], c_id);
    }

    #[test]
    fn test_find_all_paths_multiple_paths() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create diamond: A -> B -> D and A -> C -> D
        let entity_d = create_entity("D", EntityType::Function, "/src/d.ts", vec![]);
        let d_id = entity_d.id.clone();
        entities.insert(entity_d.id.clone(), entity_d);

        let import_d = ImportInfo::new("D".to_string(), "/src/d.ts".to_string());
        let entity_b = create_entity(
            "B",
            EntityType::Function,
            "/src/b.ts",
            vec![import_d.clone()],
        );
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![import_d]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_a = create_entity(
            "A",
            EntityType::Function,
            "/src/a.ts",
            vec![import_b, import_c],
        );
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);
        let paths = graph.find_all_paths(&a_id, &d_id, 100, 100);

        assert_eq!(paths.len(), 2);

        // Both paths should have length 3
        for path in &paths {
            assert_eq!(path.len(), 3);
            assert_eq!(path[0], a_id);
            assert_eq!(path[2], d_id);
        }

        // One path goes through B, other through C
        let middle_nodes: Vec<&String> = paths.iter().map(|p| &p[1]).collect();
        assert!(middle_nodes.contains(&&b_id));
        assert!(middle_nodes.contains(&&c_id));
    }

    #[test]
    fn test_find_all_paths_no_path() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // A and B are not connected
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let graph = DependencyGraph::from_entities(&entities);
        let paths = graph.find_all_paths(&a_id, &b_id, 100, 100);

        assert!(paths.is_empty());
    }

    #[test]
    fn test_find_all_paths_handles_cycles() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create: A -> B -> C, A -> C (diamond with potential cycle)
        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_b = create_entity(
            "B",
            EntityType::Function,
            "/src/b.ts",
            vec![import_c.clone()],
        );
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let entity_a = create_entity(
            "A",
            EntityType::Function,
            "/src/a.ts",
            vec![import_b, import_c],
        );
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);
        let paths = graph.find_all_paths(&a_id, &c_id, 100, 100);

        // Should find both A -> B -> C and A -> C
        assert_eq!(paths.len(), 2);

        let path_lengths: Vec<usize> = paths.iter().map(|p| p.len()).collect();
        assert!(path_lengths.contains(&2)); // A -> C
        assert!(path_lengths.contains(&3)); // A -> B -> C
    }

    #[test]
    fn test_find_all_paths_respects_max_limit() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create diamond: A -> B -> D and A -> C -> D (2 paths)
        let entity_d = create_entity("D", EntityType::Function, "/src/d.ts", vec![]);
        let d_id = entity_d.id.clone();
        entities.insert(entity_d.id.clone(), entity_d);

        let import_d = ImportInfo::new("D".to_string(), "/src/d.ts".to_string());
        let entity_b = create_entity(
            "B",
            EntityType::Function,
            "/src/b.ts",
            vec![import_d.clone()],
        );
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![import_d]);
        entities.insert(entity_c.id.clone(), entity_c);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_a = create_entity(
            "A",
            EntityType::Function,
            "/src/a.ts",
            vec![import_b, import_c],
        );
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);

        // Limit to 1 path
        let paths = graph.find_all_paths(&a_id, &d_id, 1, 100);
        assert_eq!(paths.len(), 1);

        // Allow all paths
        let paths = graph.find_all_paths(&a_id, &d_id, 100, 100);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_find_all_paths_respects_max_depth() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create chain: A -> B -> C -> D (4 nodes, path length 4)
        let entity_d = create_entity("D", EntityType::Function, "/src/d.ts", vec![]);
        let d_id = entity_d.id.clone();
        entities.insert(entity_d.id.clone(), entity_d);

        let import_d = ImportInfo::new("D".to_string(), "/src/d.ts".to_string());
        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![import_d]);
        entities.insert(entity_c.id.clone(), entity_c);

        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![import_c]);
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![import_b]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);

        // Depth 3 should not find the path (A -> B -> C -> D is length 4)
        let paths = graph.find_all_paths(&a_id, &d_id, 100, 3);
        assert!(paths.is_empty());

        // Depth 4 should find the path
        let paths = graph.find_all_paths(&a_id, &d_id, 100, 4);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].len(), 4);
    }

    #[test]
    fn test_find_cycles_no_cycles() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // A -> B -> C (no cycles)
        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        entities.insert(entity_c.id.clone(), entity_c);

        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![import_c]);
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![import_b]);
        entities.insert(entity_a.id.clone(), entity_a);

        let graph = DependencyGraph::from_entities(&entities);
        let cycles = graph.find_cycles(100, 100);

        assert!(cycles.is_empty());
    }

    #[test]
    fn test_find_cycles_simple_cycle() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create A -> B -> A cycle
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);

        let graph = DependencyGraph::from_entities(&entities);
        let cycles = graph.find_cycles(100, 100);

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn test_find_cycles_three_node_cycle() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create A -> B -> C -> A cycle
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_c]);
        entities.get_mut(&c_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);

        let graph = DependencyGraph::from_entities(&entities);
        let cycles = graph.find_cycles(100, 100);

        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }

    #[test]
    fn test_find_cycles_respects_max_cycles() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create two separate cycles: A -> B -> A and C -> D -> C
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let entity_d = create_entity("D", EntityType::Function, "/src/d.ts", vec![]);
        let d_id = entity_d.id.clone();
        entities.insert(entity_d.id.clone(), entity_d);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());
        let import_d = ImportInfo::new("D".to_string(), "/src/d.ts".to_string());
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);
        entities.get_mut(&c_id).unwrap().deps = std::rc::Rc::new(vec![import_d]);
        entities.get_mut(&d_id).unwrap().deps = std::rc::Rc::new(vec![import_c]);

        let graph = DependencyGraph::from_entities(&entities);

        // Limit to 1 cycle
        let cycles = graph.find_cycles(1, 100);
        assert_eq!(cycles.len(), 1);

        // Allow all cycles
        let cycles = graph.find_cycles(100, 100);
        assert_eq!(cycles.len(), 2);
    }

    #[test]
    fn test_find_cycles_respects_max_depth() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create A -> B -> C -> D -> A (4 node cycle)
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let entity_c = create_entity("C", EntityType::Function, "/src/c.ts", vec![]);
        let c_id = entity_c.id.clone();
        entities.insert(entity_c.id.clone(), entity_c);

        let entity_d = create_entity("D", EntityType::Function, "/src/d.ts", vec![]);
        let d_id = entity_d.id.clone();
        entities.insert(entity_d.id.clone(), entity_d);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_c = ImportInfo::new("C".to_string(), "/src/c.ts".to_string());
        let import_d = ImportInfo::new("D".to_string(), "/src/d.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_c]);
        entities.get_mut(&c_id).unwrap().deps = std::rc::Rc::new(vec![import_d]);
        entities.get_mut(&d_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);

        let graph = DependencyGraph::from_entities(&entities);

        // Depth 3 should not find the 4-node cycle
        let cycles = graph.find_cycles(100, 3);
        assert!(cycles.is_empty());

        // Depth 4 should find the cycle
        let cycles = graph.find_cycles(100, 4);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 4);
    }

    #[test]
    fn test_find_cycles_deduplicates() {
        let mut entities: HashMap<String, Entity> = HashMap::new();

        // Create A -> B -> A cycle - should only find it once, not twice
        let entity_a = create_entity("A", EntityType::Function, "/src/a.ts", vec![]);
        let a_id = entity_a.id.clone();
        entities.insert(entity_a.id.clone(), entity_a);

        let entity_b = create_entity("B", EntityType::Function, "/src/b.ts", vec![]);
        let b_id = entity_b.id.clone();
        entities.insert(entity_b.id.clone(), entity_b);

        let import_b = ImportInfo::new("B".to_string(), "/src/b.ts".to_string());
        let import_a = ImportInfo::new("A".to_string(), "/src/a.ts".to_string());

        entities.get_mut(&a_id).unwrap().deps = std::rc::Rc::new(vec![import_b]);
        entities.get_mut(&b_id).unwrap().deps = std::rc::Rc::new(vec![import_a]);

        let graph = DependencyGraph::from_entities(&entities);
        let cycles = graph.find_cycles(100, 100);

        // The cycle A -> B -> A is the same as B -> A -> B, should only appear once
        assert_eq!(cycles.len(), 1);
    }
}
