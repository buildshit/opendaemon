use crate::config::DmnConfig;
use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("Cyclic dependency detected: {0}")]
    CyclicDependency(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
}

pub struct ServiceGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
}

impl ServiceGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Build a ServiceGraph from a DmnConfig
    /// Creates nodes for each service and edges for dependencies
    pub fn from_config(config: &DmnConfig) -> Result<Self, GraphError> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // First pass: create nodes for all services
        for service_name in config.services.keys() {
            let node_idx = graph.add_node(service_name.clone());
            node_map.insert(service_name.clone(), node_idx);
        }

        // Second pass: create edges for dependencies
        for (service_name, service_config) in &config.services {
            let service_node = node_map[service_name];

            for dependency in &service_config.depends_on {
                // Get the dependency node
                let dep_node = node_map
                    .get(dependency)
                    .ok_or_else(|| GraphError::ServiceNotFound(dependency.clone()))?;

                // Add edge from dependency to service (dependency -> service)
                graph.add_edge(*dep_node, service_node, ());
            }
        }

        let service_graph = Self { graph, node_map };

        // Check for cycles after building the graph
        service_graph.check_cycles()?;

        Ok(service_graph)
    }

    /// Check if the graph contains any cycles
    /// Returns an error with details about the cycle if found
    pub fn check_cycles(&self) -> Result<(), GraphError> {
        if is_cyclic_directed(&self.graph) {
            // Find and report the cycle
            let cycle_services = self.find_cycle_services();
            let cycle_description = cycle_services.join(" -> ");
            return Err(GraphError::CyclicDependency(format!(
                "{}",
                cycle_description
            )));
        }
        Ok(())
    }

    /// Find services involved in a cycle using DFS
    fn find_cycle_services(&self) -> Vec<String> {
        // Try to find a cycle by doing DFS from each node
        for start_node in self.graph.node_indices() {
            let mut visited = vec![false; self.graph.node_count()];
            let mut rec_stack = vec![false; self.graph.node_count()];
            let mut path = Vec::new();

            if self.dfs_find_cycle(start_node, &mut visited, &mut rec_stack, &mut path) {
                // Found a cycle, extract service names from path
                return path.iter().map(|&idx| self.graph[idx].clone()).collect();
            }
        }

        // Shouldn't reach here if is_cyclic_directed returned true
        vec!["unknown cycle".to_string()]
    }

    /// DFS helper to find a cycle and build the path
    fn dfs_find_cycle(
        &self,
        node: NodeIndex,
        visited: &mut [bool],
        rec_stack: &mut [bool],
        path: &mut Vec<NodeIndex>,
    ) -> bool {
        let node_idx = node.index();

        if rec_stack[node_idx] {
            // Found a cycle, add current node to complete the cycle
            path.push(node);
            return true;
        }

        if visited[node_idx] {
            return false;
        }

        visited[node_idx] = true;
        rec_stack[node_idx] = true;
        path.push(node);

        // Visit all neighbors
        for neighbor in self.graph.neighbors(node) {
            if self.dfs_find_cycle(neighbor, visited, rec_stack, path) {
                return true;
            }
        }

        // Backtrack
        rec_stack[node_idx] = false;
        path.pop();
        false
    }

    /// Get the start order for services using topological sort
    /// Returns services in the order they should be started (dependencies first)
    pub fn get_start_order(&self) -> Result<Vec<String>, GraphError> {
        match toposort(&self.graph, None) {
            Ok(sorted_nodes) => {
                // Convert NodeIndex to service names
                let service_names: Vec<String> = sorted_nodes
                    .iter()
                    .map(|&node_idx| self.graph[node_idx].clone())
                    .collect();
                Ok(service_names)
            }
            Err(_) => {
                // This shouldn't happen since we check for cycles in from_config
                // but handle it gracefully
                Err(GraphError::CyclicDependency(
                    "Cannot determine start order due to circular dependencies".to_string(),
                ))
            }
        }
    }

    /// Get dependencies for a specific service
    /// Returns the list of services that this service depends on
    pub fn get_dependencies(&self, service_name: &str) -> Result<Vec<String>, GraphError> {
        let node_idx = self
            .node_map
            .get(service_name)
            .ok_or_else(|| GraphError::ServiceNotFound(service_name.to_string()))?;

        let dependencies: Vec<String> = self
            .graph
            .neighbors_directed(*node_idx, petgraph::Direction::Incoming)
            .map(|dep_idx| self.graph[dep_idx].clone())
            .collect();

        Ok(dependencies)
    }

    /// Get dependents for a specific service
    /// Returns the list of services that depend on this service
    pub fn get_dependents(&self, service_name: &str) -> Result<Vec<String>, GraphError> {
        let node_idx = self
            .node_map
            .get(service_name)
            .ok_or_else(|| GraphError::ServiceNotFound(service_name.to_string()))?;

        let dependents: Vec<String> = self
            .graph
            .neighbors_directed(*node_idx, petgraph::Direction::Outgoing)
            .map(|dep_idx| self.graph[dep_idx].clone())
            .collect();

        Ok(dependents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DmnConfig, ServiceConfig};
    use std::collections::HashMap;

    #[test]
    fn test_graph_construction_single_service() {
        let mut services = HashMap::new();
        services.insert(
            "app".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        assert_eq!(graph.node_map.len(), 1);
        assert!(graph.node_map.contains_key("app"));
        assert_eq!(graph.graph.node_count(), 1);
        assert_eq!(graph.graph.edge_count(), 0);
    }

    #[test]
    fn test_graph_construction_linear_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec!["backend".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        assert_eq!(graph.node_map.len(), 3);
        assert_eq!(graph.graph.node_count(), 3);
        assert_eq!(graph.graph.edge_count(), 2);
    }

    #[test]
    fn test_graph_construction_multiple_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "redis".to_string(),
            ServiceConfig {
                command: "redis-server".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["database".to_string(), "redis".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        assert_eq!(graph.node_map.len(), 3);
        assert_eq!(graph.graph.node_count(), 3);
        assert_eq!(graph.graph.edge_count(), 2);
    }

    #[test]
    fn test_graph_construction_missing_dependency() {
        let mut services = HashMap::new();
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec!["backend".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let result = ServiceGraph::from_config(&config);
        assert!(result.is_err());
        match result {
            Err(GraphError::ServiceNotFound(service)) => {
                assert_eq!(service, "backend");
            }
            _ => panic!("Expected ServiceNotFound error"),
        }
    }

    #[test]
    fn test_graph_construction_complex_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "cache".to_string(),
            ServiceConfig {
                command: "redis".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "api".to_string(),
            ServiceConfig {
                command: "api-server".to_string(),
                depends_on: vec!["db".to_string(), "cache".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "worker".to_string(),
            ServiceConfig {
                command: "worker".to_string(),
                depends_on: vec!["db".to_string(), "cache".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "web".to_string(),
            ServiceConfig {
                command: "web-server".to_string(),
                depends_on: vec!["api".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        assert_eq!(graph.node_map.len(), 5);
        assert_eq!(graph.graph.node_count(), 5);
        // db->api, cache->api, db->worker, cache->worker, api->web = 5 edges
        assert_eq!(graph.graph.edge_count(), 5);
    }

    #[test]
    fn test_cycle_detection_direct_cycle() {
        let mut services = HashMap::new();
        services.insert(
            "service_a".to_string(),
            ServiceConfig {
                command: "echo a".to_string(),
                depends_on: vec!["service_b".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "service_b".to_string(),
            ServiceConfig {
                command: "echo b".to_string(),
                depends_on: vec!["service_a".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let result = ServiceGraph::from_config(&config);
        assert!(result.is_err());
        match result {
            Err(GraphError::CyclicDependency(msg)) => {
                // Should contain both services in the cycle
                assert!(msg.contains("service_a") || msg.contains("service_b"));
            }
            _ => panic!("Expected CyclicDependency error"),
        }
    }

    #[test]
    fn test_cycle_detection_indirect_cycle() {
        let mut services = HashMap::new();
        services.insert(
            "a".to_string(),
            ServiceConfig {
                command: "echo a".to_string(),
                depends_on: vec!["b".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "b".to_string(),
            ServiceConfig {
                command: "echo b".to_string(),
                depends_on: vec!["c".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "c".to_string(),
            ServiceConfig {
                command: "echo c".to_string(),
                depends_on: vec!["a".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let result = ServiceGraph::from_config(&config);
        assert!(result.is_err());
        match result {
            Err(GraphError::CyclicDependency(msg)) => {
                // Should mention the cycle
                assert!(!msg.is_empty());
            }
            _ => panic!("Expected CyclicDependency error"),
        }
    }

    #[test]
    fn test_cycle_detection_self_dependency() {
        let mut services = HashMap::new();
        services.insert(
            "service".to_string(),
            ServiceConfig {
                command: "echo test".to_string(),
                depends_on: vec!["service".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let result = ServiceGraph::from_config(&config);
        assert!(result.is_err());
        match result {
            Err(GraphError::CyclicDependency(_)) => {}
            _ => panic!("Expected CyclicDependency error"),
        }
    }

    #[test]
    fn test_no_cycle_in_acyclic_graph() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["db".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec!["backend".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let result = ServiceGraph::from_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cycle_detection_complex_cycle() {
        let mut services = HashMap::new();
        services.insert(
            "a".to_string(),
            ServiceConfig {
                command: "echo a".to_string(),
                depends_on: vec!["b".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "b".to_string(),
            ServiceConfig {
                command: "echo b".to_string(),
                depends_on: vec!["c".to_string(), "d".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "c".to_string(),
            ServiceConfig {
                command: "echo c".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "d".to_string(),
            ServiceConfig {
                command: "echo d".to_string(),
                depends_on: vec!["a".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let result = ServiceGraph::from_config(&config);
        assert!(result.is_err());
        match result {
            Err(GraphError::CyclicDependency(_)) => {}
            _ => panic!("Expected CyclicDependency error"),
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec!["backend".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let start_order = graph.get_start_order().unwrap();

        assert_eq!(start_order.len(), 3);
        // Database should come before backend
        let db_pos = start_order.iter().position(|s| s == "database").unwrap();
        let backend_pos = start_order.iter().position(|s| s == "backend").unwrap();
        let frontend_pos = start_order.iter().position(|s| s == "frontend").unwrap();

        assert!(db_pos < backend_pos);
        assert!(backend_pos < frontend_pos);
    }

    #[test]
    fn test_topological_sort_parallel_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "redis".to_string(),
            ServiceConfig {
                command: "redis-server".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["db".to_string(), "redis".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let start_order = graph.get_start_order().unwrap();

        assert_eq!(start_order.len(), 3);
        // Both db and redis should come before backend
        let db_pos = start_order.iter().position(|s| s == "db").unwrap();
        let redis_pos = start_order.iter().position(|s| s == "redis").unwrap();
        let backend_pos = start_order.iter().position(|s| s == "backend").unwrap();

        assert!(db_pos < backend_pos);
        assert!(redis_pos < backend_pos);
    }

    #[test]
    fn test_topological_sort_complex() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "cache".to_string(),
            ServiceConfig {
                command: "redis".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "api".to_string(),
            ServiceConfig {
                command: "api-server".to_string(),
                depends_on: vec!["db".to_string(), "cache".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "worker".to_string(),
            ServiceConfig {
                command: "worker".to_string(),
                depends_on: vec!["db".to_string(), "cache".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "web".to_string(),
            ServiceConfig {
                command: "web-server".to_string(),
                depends_on: vec!["api".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let start_order = graph.get_start_order().unwrap();

        assert_eq!(start_order.len(), 5);

        // Verify ordering constraints
        let db_pos = start_order.iter().position(|s| s == "db").unwrap();
        let cache_pos = start_order.iter().position(|s| s == "cache").unwrap();
        let api_pos = start_order.iter().position(|s| s == "api").unwrap();
        let worker_pos = start_order.iter().position(|s| s == "worker").unwrap();
        let web_pos = start_order.iter().position(|s| s == "web").unwrap();

        // db and cache must come before api and worker
        assert!(db_pos < api_pos);
        assert!(db_pos < worker_pos);
        assert!(cache_pos < api_pos);
        assert!(cache_pos < worker_pos);

        // api must come before web
        assert!(api_pos < web_pos);
    }

    #[test]
    fn test_topological_sort_single_service() {
        let mut services = HashMap::new();
        services.insert(
            "app".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let start_order = graph.get_start_order().unwrap();

        assert_eq!(start_order.len(), 1);
        assert_eq!(start_order[0], "app");
    }

    #[test]
    fn test_get_dependencies() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "redis".to_string(),
            ServiceConfig {
                command: "redis-server".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["db".to_string(), "redis".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();

        // Test getting dependencies for backend
        let backend_deps = graph.get_dependencies("backend").unwrap();
        assert_eq!(backend_deps.len(), 2);
        assert!(backend_deps.contains(&"db".to_string()));
        assert!(backend_deps.contains(&"redis".to_string()));

        // Test getting dependencies for db (should be empty)
        let db_deps = graph.get_dependencies("db").unwrap();
        assert_eq!(db_deps.len(), 0);
    }

    #[test]
    fn test_get_dependencies_nonexistent_service() {
        let mut services = HashMap::new();
        services.insert(
            "app".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let result = graph.get_dependencies("nonexistent");

        assert!(result.is_err());
        match result {
            Err(GraphError::ServiceNotFound(service)) => {
                assert_eq!(service, "nonexistent");
            }
            _ => panic!("Expected ServiceNotFound error"),
        }
    }

    #[test]
    fn test_get_dependents_linear() {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec!["database".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "frontend".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec!["backend".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();

        // Database is depended on by backend
        let db_dependents = graph.get_dependents("database").unwrap();
        assert_eq!(db_dependents.len(), 1);
        assert!(db_dependents.contains(&"backend".to_string()));

        // Backend is depended on by frontend
        let backend_dependents = graph.get_dependents("backend").unwrap();
        assert_eq!(backend_dependents.len(), 1);
        assert!(backend_dependents.contains(&"frontend".to_string()));

        // Frontend has no dependents
        let frontend_dependents = graph.get_dependents("frontend").unwrap();
        assert_eq!(frontend_dependents.len(), 0);
    }

    #[test]
    fn test_get_dependents_multiple() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "api".to_string(),
            ServiceConfig {
                command: "api-server".to_string(),
                depends_on: vec!["db".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "worker".to_string(),
            ServiceConfig {
                command: "worker".to_string(),
                depends_on: vec!["db".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "scheduler".to_string(),
            ServiceConfig {
                command: "scheduler".to_string(),
                depends_on: vec!["db".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();

        // Database is depended on by api, worker, and scheduler
        let db_dependents = graph.get_dependents("db").unwrap();
        assert_eq!(db_dependents.len(), 3);
        assert!(db_dependents.contains(&"api".to_string()));
        assert!(db_dependents.contains(&"worker".to_string()));
        assert!(db_dependents.contains(&"scheduler".to_string()));
    }

    #[test]
    fn test_get_dependents_complex() {
        let mut services = HashMap::new();
        services.insert(
            "db".to_string(),
            ServiceConfig {
                command: "postgres".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "cache".to_string(),
            ServiceConfig {
                command: "redis".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "api".to_string(),
            ServiceConfig {
                command: "api-server".to_string(),
                depends_on: vec!["db".to_string(), "cache".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "worker".to_string(),
            ServiceConfig {
                command: "worker".to_string(),
                depends_on: vec!["db".to_string(), "cache".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "web".to_string(),
            ServiceConfig {
                command: "web-server".to_string(),
                depends_on: vec!["api".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();

        // db is depended on by api and worker
        let db_dependents = graph.get_dependents("db").unwrap();
        assert_eq!(db_dependents.len(), 2);
        assert!(db_dependents.contains(&"api".to_string()));
        assert!(db_dependents.contains(&"worker".to_string()));

        // cache is depended on by api and worker
        let cache_dependents = graph.get_dependents("cache").unwrap();
        assert_eq!(cache_dependents.len(), 2);
        assert!(cache_dependents.contains(&"api".to_string()));
        assert!(cache_dependents.contains(&"worker".to_string()));

        // api is depended on by web
        let api_dependents = graph.get_dependents("api").unwrap();
        assert_eq!(api_dependents.len(), 1);
        assert!(api_dependents.contains(&"web".to_string()));

        // worker has no dependents
        let worker_dependents = graph.get_dependents("worker").unwrap();
        assert_eq!(worker_dependents.len(), 0);

        // web has no dependents
        let web_dependents = graph.get_dependents("web").unwrap();
        assert_eq!(web_dependents.len(), 0);
    }

    #[test]
    fn test_get_dependents_no_dependents() {
        let mut services = HashMap::new();
        services.insert(
            "app".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let dependents = graph.get_dependents("app").unwrap();

        assert_eq!(dependents.len(), 0);
    }

    #[test]
    fn test_get_dependents_nonexistent_service() {
        let mut services = HashMap::new();
        services.insert(
            "app".to_string(),
            ServiceConfig {
                command: "npm start".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();
        let result = graph.get_dependents("nonexistent");

        assert!(result.is_err());
        match result {
            Err(GraphError::ServiceNotFound(service)) => {
                assert_eq!(service, "nonexistent");
            }
            _ => panic!("Expected ServiceNotFound error"),
        }
    }

    #[test]
    fn test_get_dependents_transitive() {
        // Test that get_dependents only returns direct dependents, not transitive
        let mut services = HashMap::new();
        services.insert(
            "a".to_string(),
            ServiceConfig {
                command: "echo a".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "b".to_string(),
            ServiceConfig {
                command: "echo b".to_string(),
                depends_on: vec!["a".to_string()],
                ready_when: None,
                env_file: None,
            },
        );
        services.insert(
            "c".to_string(),
            ServiceConfig {
                command: "echo c".to_string(),
                depends_on: vec!["b".to_string()],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let graph = ServiceGraph::from_config(&config).unwrap();

        // 'a' should only have 'b' as a direct dependent, not 'c'
        let a_dependents = graph.get_dependents("a").unwrap();
        assert_eq!(a_dependents.len(), 1);
        assert!(a_dependents.contains(&"b".to_string()));
        assert!(!a_dependents.contains(&"c".to_string()));
    }
}
