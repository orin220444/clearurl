use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// generate default value for the `sub` field
fn default_subdomain() -> Option<Vec<String>> {
    None
}

#[derive(Serialize, Deserialize, Debug)]
struct ConfigData {
    #[serde(default = "default_subdomain")]
    sub: Option<Vec<String>>,
    #[serde(default)]
    redirect: bool,
    #[serde(default)]
    ban: Vec<String>,
}

/// RuntimeRules expanded the configuration to actually needed rules.
#[derive(Debug)]
pub struct RuntimeRules(HashMap<String, Rule>);

impl RuntimeRules {
    pub fn get(&self, base: &str) -> Option<&Rule> {
        self.0.get(base)
    }
}

/// Represent rule for a single domain.
#[derive(Clone, Debug)]
pub struct Rule {
    pub redirect: bool,
    pub rules: Vec<regex::Regex>,
}

/// Expand the configuration to runtime data.
///
/// # Error
///
/// Panic if the the `ban` field is a invalid regex rule.
fn expand(config: HashMap<String, ConfigData>) -> RuntimeRules {
    let mut rules = HashMap::new();
    for (base, data) in config {
        let r = Rule {
            redirect: data.redirect,
            rules: data
                .ban
                .into_iter()
                .map(|re| {
                    // Use `unwrap_or_else()` instead of `expect` to avoid overhead
                    regex::Regex::new(&re)
                        .unwrap_or_else(|error| panic!("Invalid regexp: {re}\n\n\t{error}"))
                })
                .collect(),
        };

        if let Some(sub) = data.sub {
            for s in sub {
                // FIXME: Can we clone a reference not the data.
                // Try use Arc? We should take care of the self-reference bug.
                // Also we are going to be a async lib, Send + Sync is also required.
                rules.insert(format!("{s}.{base}"), r.clone());
            }
        } else {
            rules.insert(base, r);
        }
    }

    RuntimeRules(rules)
}

/// Parse rules configuration file from given `location`.
///
/// # Error
///
/// Panic if
///   * fail to read the file content
///   * fail to parse content into expected struct
///   * regexp is invalid
pub fn parse(location: &std::path::Path) -> RuntimeRules {
    let content = std::fs::read(location).unwrap_or_else(|error| {
        // HINT: Some OS might allow non-UTF-8 string as file path, so location.to_str().unwrap()
        // might panic when someone compile this program on those OS.
        panic!("fail to read from {}: {error}", location.to_str().unwrap())
    });

    let config: HashMap<String, ConfigData> = toml::from_slice(&content).unwrap_or_else(|error| {
        // Panic with full content when user set RUST_LOG=TRACE
        let trace = std::env::var("RUST_LOG");
        if let Ok(var) = trace {
            if var.to_uppercase() == "TRACE" {
                panic!(
                    "fail to parse rules content: {error}\n\nFull Content: {:#?}",
                    String::from_utf8(content)
                )
            }
        }

        panic!("fail to parse rules content: {error}")
    });

    expand(config)
}
