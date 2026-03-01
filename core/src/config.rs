use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct DmnConfig {
    pub version: String,
    pub services: HashMap<String, ServiceConfig>,
}

impl fmt::Display for DmnConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DmnConfig {{")?;
        writeln!(f, "  version: '{}'", self.version)?;
        writeln!(f, "  services: {{")?;
        for (name, config) in &self.services {
            writeln!(f, "    '{}': {}", name, config)?;
        }
        write!(f, "  }}\n}}")
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
    pub command: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub ready_when: Option<ReadyCondition>,
    pub env_file: Option<String>,
}

impl fmt::Display for ServiceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ServiceConfig {{ command: '{}', depends_on: [",
            self.command
        )?;
        for (i, dep) in self.depends_on.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "'{}'", dep)?;
        }
        write!(f, "]")?;
        if let Some(ready) = &self.ready_when {
            write!(f, ", ready_when: {}", ready)?;
        }
        if let Some(env) = &self.env_file {
            write!(f, ", env_file: '{}'", env)?;
        }
        write!(f, " }}")
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReadyCondition {
    LogContains {
        pattern: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_seconds: Option<u64>,
    },
    UrlResponds {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_seconds: Option<u64>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TaggedReadyCondition {
    LogContains {
        pattern: String,
        #[serde(default)]
        timeout_seconds: Option<u64>,
    },
    UrlResponds {
        url: String,
        #[serde(default)]
        timeout_seconds: Option<u64>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ReadyConditionRepr {
    Tagged(TaggedReadyCondition),
    LegacyLogContains {
        log_contains: String,
        #[serde(default)]
        timeout_seconds: Option<u64>,
    },
    LegacyUrlResponds {
        url_responds: String,
        #[serde(default)]
        timeout_seconds: Option<u64>,
    },
}

impl<'de> Deserialize<'de> for ReadyCondition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = ReadyConditionRepr::deserialize(deserializer)?;
        Ok(match repr {
            ReadyConditionRepr::Tagged(TaggedReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            }) => ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            },
            ReadyConditionRepr::Tagged(TaggedReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            }) => ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            },
            ReadyConditionRepr::LegacyLogContains {
                log_contains,
                timeout_seconds,
            } => ReadyCondition::LogContains {
                pattern: log_contains,
                timeout_seconds,
            },
            ReadyConditionRepr::LegacyUrlResponds {
                url_responds,
                timeout_seconds,
            } => ReadyCondition::UrlResponds {
                url: url_responds,
                timeout_seconds,
            },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Unvisited,
    Visiting,
    Visited,
}

impl fmt::Display for ReadyCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                write!(f, "log_contains: '{}'", pattern)?;
                if let Some(timeout) = timeout_seconds {
                    write!(f, " (timeout: {}s)", timeout)?;
                }
                Ok(())
            }
            ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            } => {
                write!(f, "url_responds: '{}'", url)?;
                if let Some(timeout) = timeout_seconds {
                    write!(f, " (timeout: {}s)", timeout)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

impl DmnConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: DmnConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Validate the configuration for correctness
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check version format
        if self.version.is_empty() {
            return Err(ConfigError::Validation(
                "version field cannot be empty".to_string(),
            ));
        }

        // Check that services exist
        if self.services.is_empty() {
            return Err(ConfigError::Validation(
                "at least one service must be defined".to_string(),
            ));
        }

        // Validate each service
        for (name, service) in &self.services {
            // Check command is not empty
            if service.command.trim().is_empty() {
                return Err(ConfigError::Validation(format!(
                    "service '{}': command cannot be empty",
                    name
                )));
            }

            // Check dependencies exist
            for dep in &service.depends_on {
                if !self.services.contains_key(dep) {
                    return Err(ConfigError::Validation(format!(
                        "service '{}': dependency '{}' does not exist",
                        name, dep
                    )));
                }
            }

            // Validate ready_when conditions
            if let Some(ready) = &service.ready_when {
                match ready {
                    ReadyCondition::LogContains {
                        pattern,
                        timeout_seconds,
                    } => {
                        if pattern.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "service '{}': log_contains pattern cannot be empty",
                                name
                            )));
                        }
                        // Validate regex pattern
                        if let Err(e) = regex::Regex::new(pattern) {
                            return Err(ConfigError::Validation(format!(
                                "service '{}': invalid regex pattern '{}': {}",
                                name, pattern, e
                            )));
                        }
                        // Validate timeout if specified
                        if let Some(timeout) = timeout_seconds {
                            if *timeout == 0 {
                                return Err(ConfigError::Validation(format!(
                                    "service '{}': timeout_seconds must be greater than 0",
                                    name
                                )));
                            }
                        }
                    }
                    ReadyCondition::UrlResponds {
                        url,
                        timeout_seconds,
                    } => {
                        if url.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "service '{}': url_responds url cannot be empty",
                                name
                            )));
                        }
                        // Validate timeout if specified
                        if let Some(timeout) = timeout_seconds {
                            if *timeout == 0 {
                                return Err(ConfigError::Validation(format!(
                                    "service '{}': timeout_seconds must be greater than 0",
                                    name
                                )));
                            }
                        }
                    }
                }
            }
        }

        // Check for circular dependencies
        self.check_circular_dependencies()?;

        Ok(())
    }

    /// Check for circular dependencies in the service graph
    fn check_circular_dependencies(&self) -> Result<(), ConfigError> {
        let mut states: HashMap<String, VisitState> = self
            .services
            .keys()
            .map(|name| (name.clone(), VisitState::Unvisited))
            .collect();
        let mut stack: Vec<String> = Vec::new();

        for service_name in self.services.keys() {
            if states.get(service_name) == Some(&VisitState::Unvisited) {
                self.visit_for_cycle_detection(service_name, &mut states, &mut stack)?;
            }
        }

        Ok(())
    }

    fn visit_for_cycle_detection(
        &self,
        current: &str,
        states: &mut HashMap<String, VisitState>,
        stack: &mut Vec<String>,
    ) -> Result<(), ConfigError> {
        states.insert(current.to_string(), VisitState::Visiting);
        stack.push(current.to_string());

        if let Some(service) = self.services.get(current) {
            for dep in &service.depends_on {
                let dep_state = states.get(dep).copied().unwrap_or(VisitState::Unvisited);

                match dep_state {
                    VisitState::Visited => {}
                    VisitState::Unvisited => {
                        self.visit_for_cycle_detection(dep, states, stack)?;
                    }
                    VisitState::Visiting => {
                        let cycle_start = stack.iter().position(|name| name == dep).unwrap_or(0);
                        let mut cycle_path: Vec<String> = stack[cycle_start..].to_vec();
                        cycle_path.push(dep.clone());
                        return Err(ConfigError::Validation(format!(
                            "circular dependency detected: {}",
                            cycle_path.join(" -> ")
                        )));
                    }
                }
            }
        }

        stack.pop();
        states.insert(current.to_string(), VisitState::Visited);
        Ok(())
    }
}

/// Parse a dmn.json configuration file
pub fn parse_config(path: &Path) -> Result<DmnConfig, ConfigError> {
    let config = DmnConfig::from_file(path)?;
    config.validate()?;
    Ok(config)
}

/// Load environment variables from a .env file
/// Returns a HashMap of key-value pairs
/// Handles missing files gracefully by returning an empty HashMap
pub fn load_env_file(path: &Path) -> Result<HashMap<String, String>, ConfigError> {
    // If file doesn't exist, return empty map (not an error)
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(path)?;
    let mut env_vars = HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=VALUE format
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim();

            // Validate key is not empty
            if key.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "{}:{}: environment variable key cannot be empty",
                    path.display(),
                    line_num + 1
                )));
            }

            // Remove surrounding quotes from value if present
            let value = if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                &value[1..value.len() - 1]
            } else {
                value
            };

            env_vars.insert(key.to_string(), value.to_string());
        } else {
            return Err(ConfigError::Validation(format!(
                "{}:{}: invalid line format, expected KEY=VALUE",
                path.display(),
                line_num + 1
            )));
        }
    }

    Ok(env_vars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ready_condition_log_contains_serialization() {
        let condition = ReadyCondition::LogContains {
            pattern: "Server started".to_string(),
            timeout_seconds: None,
        };

        let json = serde_json::to_value(&condition).unwrap();
        assert_eq!(json["type"], "log_contains");
        assert_eq!(json["pattern"], "Server started");
    }

    #[test]
    fn test_ready_condition_log_contains_deserialization() {
        let json = json!({
            "type": "log_contains",
            "pattern": "Ready to accept connections"
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Ready to accept connections");
                assert_eq!(timeout_seconds, None);
            }
            _ => panic!("Expected LogContains variant"),
        }
    }

    #[test]
    fn test_ready_condition_url_responds_serialization() {
        let condition = ReadyCondition::UrlResponds {
            url: "http://localhost:3000/health".to_string(),
            timeout_seconds: None,
        };

        let json = serde_json::to_value(&condition).unwrap();
        assert_eq!(json["type"], "url_responds");
        assert_eq!(json["url"], "http://localhost:3000/health");
    }

    #[test]
    fn test_ready_condition_url_responds_deserialization() {
        let json = json!({
            "type": "url_responds",
            "url": "http://localhost:8080/api/health"
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            } => {
                assert_eq!(url, "http://localhost:8080/api/health");
                assert_eq!(timeout_seconds, None);
            }
            _ => panic!("Expected UrlResponds variant"),
        }
    }

    #[test]
    fn test_ready_condition_legacy_log_contains_deserialization() {
        let json = json!({
            "log_contains": "Ready to accept connections",
            "timeout_seconds": 90
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Ready to accept connections");
                assert_eq!(timeout_seconds, Some(90));
            }
            _ => panic!("Expected LogContains variant"),
        }
    }

    #[test]
    fn test_ready_condition_legacy_url_responds_deserialization() {
        let json = json!({
            "url_responds": "http://localhost:3000/health",
            "timeout_seconds": 45
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            } => {
                assert_eq!(url, "http://localhost:3000/health");
                assert_eq!(timeout_seconds, Some(45));
            }
            _ => panic!("Expected UrlResponds variant"),
        }
    }

    #[test]
    fn test_ready_condition_display() {
        let log_condition = ReadyCondition::LogContains {
            pattern: "Started".to_string(),
            timeout_seconds: None,
        };
        assert_eq!(format!("{}", log_condition), "log_contains: 'Started'");

        let url_condition = ReadyCondition::UrlResponds {
            url: "http://localhost:3000".to_string(),
            timeout_seconds: None,
        };
        assert_eq!(
            format!("{}", url_condition),
            "url_responds: 'http://localhost:3000'"
        );
    }

    #[test]
    fn test_service_config_minimal_serialization() {
        let config = ServiceConfig {
            command: "npm start".to_string(),
            depends_on: vec![],
            ready_when: None,
            env_file: None,
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["command"], "npm start");
        assert_eq!(json["depends_on"], json!([]));
    }

    #[test]
    fn test_service_config_full_serialization() {
        let config = ServiceConfig {
            command: "cargo run".to_string(),
            depends_on: vec!["database".to_string(), "redis".to_string()],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Listening on".to_string(),
                timeout_seconds: None,
            }),
            env_file: Some(".env.local".to_string()),
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["command"], "cargo run");
        assert_eq!(json["depends_on"], json!(["database", "redis"]));
        assert_eq!(json["ready_when"]["type"], "log_contains");
        assert_eq!(json["env_file"], ".env.local");
    }

    #[test]
    fn test_service_config_deserialization_minimal() {
        let json = json!({
            "command": "python app.py"
        });

        let config: ServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.command, "python app.py");
        assert_eq!(config.depends_on.len(), 0);
        assert!(config.ready_when.is_none());
        assert!(config.env_file.is_none());
    }

    #[test]
    fn test_service_config_deserialization_full() {
        let json = json!({
            "command": "node server.js",
            "depends_on": ["postgres"],
            "ready_when": {
                "type": "url_responds",
                "url": "http://localhost:3000/health"
            },
            "env_file": ".env"
        });

        let config: ServiceConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.command, "node server.js");
        assert_eq!(config.depends_on, vec!["postgres"]);
        assert!(config.ready_when.is_some());
        assert_eq!(config.env_file, Some(".env".to_string()));
    }

    #[test]
    fn test_service_config_display() {
        let config = ServiceConfig {
            command: "npm start".to_string(),
            depends_on: vec!["db".to_string()],
            ready_when: Some(ReadyCondition::LogContains {
                pattern: "Ready".to_string(),
                timeout_seconds: None,
            }),
            env_file: Some(".env".to_string()),
        };

        let display = format!("{}", config);
        assert!(display.contains("npm start"));
        assert!(display.contains("'db'"));
        assert!(display.contains("log_contains"));
        assert!(display.contains(".env"));
    }

    #[test]
    fn test_dmn_config_serialization() {
        let mut services = HashMap::new();
        services.insert(
            "backend".to_string(),
            ServiceConfig {
                command: "cargo run".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["version"], "1.0");
        assert!(json["services"]["backend"].is_object());
    }

    #[test]
    fn test_dmn_config_deserialization() {
        let json = json!({
            "version": "1.0",
            "services": {
                "frontend": {
                    "command": "npm run dev",
                    "depends_on": ["backend"]
                },
                "backend": {
                    "command": "cargo run"
                }
            }
        });

        let config: DmnConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.services.len(), 2);
        assert!(config.services.contains_key("frontend"));
        assert!(config.services.contains_key("backend"));

        let frontend = config.services.get("frontend").unwrap();
        assert_eq!(frontend.command, "npm run dev");
        assert_eq!(frontend.depends_on, vec!["backend"]);
    }

    #[test]
    fn test_dmn_config_display() {
        let mut services = HashMap::new();
        services.insert(
            "app".to_string(),
            ServiceConfig {
                command: "run.sh".to_string(),
                depends_on: vec![],
                ready_when: None,
                env_file: None,
            },
        );

        let config = DmnConfig {
            version: "1.0".to_string(),
            services,
        };

        let display = format!("{}", config);
        assert!(display.contains("DmnConfig"));
        assert!(display.contains("1.0"));
        assert!(display.contains("app"));
    }

    #[test]
    fn test_round_trip_serialization() {
        let json = json!({
            "version": "1.0",
            "services": {
                "web": {
                    "command": "npm start",
                    "depends_on": ["api", "db"],
                    "ready_when": {
                        "type": "log_contains",
                        "pattern": "Server listening"
                    },
                    "env_file": ".env.production"
                }
            }
        });

        let config: DmnConfig = serde_json::from_value(json.clone()).unwrap();
        let serialized = serde_json::to_value(&config).unwrap();

        assert_eq!(json, serialized);
    }

    #[test]
    fn test_parse_config_valid() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_valid_config.json");

        let config_content = r#"{
            "version": "1.0",
            "services": {
                "backend": {
                    "command": "cargo run"
                }
            }
        }"#;

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let result = parse_config(&config_path);
        assert!(result.is_ok());

        std::fs::remove_file(config_path).ok();
    }

    #[test]
    fn test_parse_config_invalid_json() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_invalid_json.json");

        let config_content = r#"{
            "version": "1.0",
            "services": {
                "backend": {
                    "command": "cargo run"
                }
            
        }"#; // Missing closing brace

        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let result = parse_config(&config_path);
        assert!(result.is_err());

        std::fs::remove_file(config_path).ok();
    }

    #[test]
    fn test_parse_config_missing_file() {
        let config_path = std::path::PathBuf::from("/nonexistent/path/dmn.json");
        let result = parse_config(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_version() {
        let config = DmnConfig {
            version: "".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "echo test".to_string(),
                        depends_on: vec![],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("version"));
    }

    #[test]
    fn test_validate_no_services() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: HashMap::new(),
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least one service"));
    }

    #[test]
    fn test_validate_empty_command() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "   ".to_string(),
                        depends_on: vec![],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("command cannot be empty"));
    }

    #[test]
    fn test_validate_missing_dependency() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "frontend".to_string(),
                    ServiceConfig {
                        command: "npm start".to_string(),
                        depends_on: vec!["backend".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("dependency 'backend' does not exist"));
    }

    #[test]
    fn test_validate_empty_log_pattern() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "echo test".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::LogContains {
                            pattern: "".to_string(),
                            timeout_seconds: None,
                        }),
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("pattern cannot be empty"));
    }

    #[test]
    fn test_validate_invalid_regex() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "echo test".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::LogContains {
                            pattern: "[invalid(".to_string(),
                            timeout_seconds: None,
                        }),
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid regex pattern"));
    }

    #[test]
    fn test_validate_empty_url() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "echo test".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::UrlResponds {
                            url: "".to_string(),
                            timeout_seconds: None,
                        }),
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("url cannot be empty"));
    }

    #[test]
    fn test_validate_circular_dependency_direct() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "service_a".to_string(),
                    ServiceConfig {
                        command: "echo a".to_string(),
                        depends_on: vec!["service_b".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map.insert(
                    "service_b".to_string(),
                    ServiceConfig {
                        command: "echo b".to_string(),
                        depends_on: vec!["service_a".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("circular dependency"));
    }

    #[test]
    fn test_validate_circular_dependency_indirect() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "a".to_string(),
                    ServiceConfig {
                        command: "echo a".to_string(),
                        depends_on: vec!["b".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map.insert(
                    "b".to_string(),
                    ServiceConfig {
                        command: "echo b".to_string(),
                        depends_on: vec!["c".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map.insert(
                    "c".to_string(),
                    ServiceConfig {
                        command: "echo c".to_string(),
                        depends_on: vec!["a".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("circular dependency"));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "database".to_string(),
                    ServiceConfig {
                        command: "postgres".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::LogContains {
                            pattern: "ready to accept connections".to_string(),
                            timeout_seconds: None,
                        }),
                        env_file: None,
                    },
                );
                map.insert(
                    "backend".to_string(),
                    ServiceConfig {
                        command: "cargo run".to_string(),
                        depends_on: vec!["database".to_string()],
                        ready_when: Some(ReadyCondition::UrlResponds {
                            url: "http://localhost:8080/health".to_string(),
                            timeout_seconds: None,
                        }),
                        env_file: Some(".env".to_string()),
                    },
                );
                map.insert(
                    "frontend".to_string(),
                    ServiceConfig {
                        command: "npm start".to_string(),
                        depends_on: vec!["backend".to_string()],
                        ready_when: None,
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ready_condition_with_timeout_serialization() {
        let log_condition = ReadyCondition::LogContains {
            pattern: "Server started".to_string(),
            timeout_seconds: Some(120),
        };

        let json = serde_json::to_value(&log_condition).unwrap();
        assert_eq!(json["type"], "log_contains");
        assert_eq!(json["pattern"], "Server started");
        assert_eq!(json["timeout_seconds"], 120);

        let url_condition = ReadyCondition::UrlResponds {
            url: "http://localhost:3000/health".to_string(),
            timeout_seconds: Some(90),
        };

        let json = serde_json::to_value(&url_condition).unwrap();
        assert_eq!(json["type"], "url_responds");
        assert_eq!(json["url"], "http://localhost:3000/health");
        assert_eq!(json["timeout_seconds"], 90);
    }

    #[test]
    fn test_ready_condition_with_timeout_deserialization() {
        // Test log_contains with timeout
        let json = json!({
            "type": "log_contains",
            "pattern": "Ready",
            "timeout_seconds": 60
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Ready");
                assert_eq!(timeout_seconds, Some(60));
            }
            _ => panic!("Expected LogContains variant"),
        }

        // Test url_responds with timeout
        let json = json!({
            "type": "url_responds",
            "url": "http://localhost:8080",
            "timeout_seconds": 45
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::UrlResponds {
                url,
                timeout_seconds,
            } => {
                assert_eq!(url, "http://localhost:8080");
                assert_eq!(timeout_seconds, Some(45));
            }
            _ => panic!("Expected UrlResponds variant"),
        }
    }

    #[test]
    fn test_ready_condition_backward_compatibility() {
        // Test that old configs without timeout_seconds still work
        let json = json!({
            "type": "log_contains",
            "pattern": "Started"
        });

        let condition: ReadyCondition = serde_json::from_value(json).unwrap();
        match condition {
            ReadyCondition::LogContains {
                pattern,
                timeout_seconds,
            } => {
                assert_eq!(pattern, "Started");
                assert_eq!(timeout_seconds, None);
            }
            _ => panic!("Expected LogContains variant"),
        }
    }

    #[test]
    fn test_ready_condition_display_with_timeout() {
        let log_condition = ReadyCondition::LogContains {
            pattern: "Started".to_string(),
            timeout_seconds: Some(120),
        };
        assert_eq!(
            format!("{}", log_condition),
            "log_contains: 'Started' (timeout: 120s)"
        );

        let url_condition = ReadyCondition::UrlResponds {
            url: "http://localhost:3000".to_string(),
            timeout_seconds: Some(90),
        };
        assert_eq!(
            format!("{}", url_condition),
            "url_responds: 'http://localhost:3000' (timeout: 90s)"
        );
    }

    #[test]
    fn test_validate_zero_timeout() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "echo test".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::LogContains {
                            pattern: "Ready".to_string(),
                            timeout_seconds: Some(0),
                        }),
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timeout_seconds must be greater than 0"));
    }

    #[test]
    fn test_validate_zero_timeout_url_responds() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "test".to_string(),
                    ServiceConfig {
                        command: "echo test".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::UrlResponds {
                            url: "http://localhost:3000".to_string(),
                            timeout_seconds: Some(0),
                        }),
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timeout_seconds must be greater than 0"));
    }

    #[test]
    fn test_validate_valid_timeout() {
        let config = DmnConfig {
            version: "1.0".to_string(),
            services: {
                let mut map = HashMap::new();
                map.insert(
                    "slow_service".to_string(),
                    ServiceConfig {
                        command: "slow_start.sh".to_string(),
                        depends_on: vec![],
                        ready_when: Some(ReadyCondition::LogContains {
                            pattern: "Ready".to_string(),
                            timeout_seconds: Some(300),
                        }),
                        env_file: None,
                    },
                );
                map
            },
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_env_file_missing() {
        let path = std::path::PathBuf::from("/nonexistent/.env");
        let result = load_env_file(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_load_env_file_basic() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_basic.env");

        let env_content = r#"DATABASE_URL=postgres://localhost/mydb
API_KEY=secret123
PORT=8080"#;

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_ok());

        let env_vars = result.unwrap();
        assert_eq!(env_vars.len(), 3);
        assert_eq!(
            env_vars.get("DATABASE_URL"),
            Some(&"postgres://localhost/mydb".to_string())
        );
        assert_eq!(env_vars.get("API_KEY"), Some(&"secret123".to_string()));
        assert_eq!(env_vars.get("PORT"), Some(&"8080".to_string()));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_with_quotes() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_quotes.env");

        let env_content = r#"SINGLE='value with spaces'
DOUBLE="another value"
NO_QUOTES=plain"#;

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_ok());

        let env_vars = result.unwrap();
        assert_eq!(
            env_vars.get("SINGLE"),
            Some(&"value with spaces".to_string())
        );
        assert_eq!(env_vars.get("DOUBLE"), Some(&"another value".to_string()));
        assert_eq!(env_vars.get("NO_QUOTES"), Some(&"plain".to_string()));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_with_comments() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_comments.env");

        let env_content = r#"# This is a comment
KEY1=value1
# Another comment
KEY2=value2

# Empty lines are ignored
KEY3=value3"#;

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_ok());

        let env_vars = result.unwrap();
        assert_eq!(env_vars.len(), 3);
        assert_eq!(env_vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env_vars.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env_vars.get("KEY3"), Some(&"value3".to_string()));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_with_whitespace() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_whitespace.env");

        let env_content = r#"  KEY1  =  value1  
KEY2=value2
  KEY3=value3  "#;

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_ok());

        let env_vars = result.unwrap();
        assert_eq!(env_vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env_vars.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env_vars.get("KEY3"), Some(&"value3".to_string()));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_empty_key() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_empty_key.env");

        let env_content = "=value";

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("key cannot be empty"));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_invalid_format() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_invalid_format.env");

        let env_content = "INVALID_LINE_WITHOUT_EQUALS";

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid line format"));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_empty_value() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_empty_value.env");

        let env_content = r#"KEY1=
KEY2=""
KEY3=''"#;

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_ok());

        let env_vars = result.unwrap();
        assert_eq!(env_vars.get("KEY1"), Some(&"".to_string()));
        assert_eq!(env_vars.get("KEY2"), Some(&"".to_string()));
        assert_eq!(env_vars.get("KEY3"), Some(&"".to_string()));

        std::fs::remove_file(env_path).ok();
    }

    #[test]
    fn test_load_env_file_special_characters() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("test_special_chars.env");

        let env_content = r#"URL=https://example.com/path?query=value&other=123
JSON={"key":"value"}
PATH=/usr/local/bin:/usr/bin"#;

        let mut file = std::fs::File::create(&env_path).unwrap();
        file.write_all(env_content.as_bytes()).unwrap();

        let result = load_env_file(&env_path);
        assert!(result.is_ok());

        let env_vars = result.unwrap();
        assert_eq!(
            env_vars.get("URL"),
            Some(&"https://example.com/path?query=value&other=123".to_string())
        );
        assert_eq!(
            env_vars.get("JSON"),
            Some(&r#"{"key":"value"}"#.to_string())
        );
        assert_eq!(
            env_vars.get("PATH"),
            Some(&"/usr/local/bin:/usr/bin".to_string())
        );

        std::fs::remove_file(env_path).ok();
    }
}
