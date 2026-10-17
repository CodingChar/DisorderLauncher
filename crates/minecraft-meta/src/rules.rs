use crate::Rule;
use std::collections::HashSet;

pub fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    }
}

/// Features habilitadas para la evaluacion de reglas (`is_demo_user`,
/// `has_custom_resolution`, ...). Una feature ausente se considera desactivada.
#[derive(Debug, Clone, Default)]
pub struct RuleFeatures {
    enabled: HashSet<String>,
}

impl RuleFeatures {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with(mut self, feature: &str) -> Self {
        self.enabled.insert(feature.to_string());
        self
    }

    fn is_enabled(&self, feature: &str) -> bool {
        self.enabled.contains(feature)
    }
}

/// Evalua una lista de reglas al estilo Mojang: sin reglas se permite; con reglas
/// el resultado parte en "denegado" y la ultima regla que aplica decide.
///
/// Las condiciones `os.version` no se evaluan: una regla que las exige se
/// considera no aplicable.
pub fn rules_allow_with(rules: &[Rule], features: &RuleFeatures) -> bool {
    if rules.is_empty() {
        return true;
    }

    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule, features) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

pub fn rules_allow(rules: &[Rule]) -> bool {
    rules_allow_with(rules, &RuleFeatures::none())
}

fn rule_matches(rule: &Rule, features: &RuleFeatures) -> bool {
    if let Some(os) = &rule.os {
        if os.name.as_deref().is_some_and(|n| n != current_os_name()) {
            return false;
        }
        if os.arch.as_deref().is_some_and(|a| a != current_arch()) {
            return false;
        }
        if os.version.is_some() {
            return false;
        }
    }

    if let Some(required) = &rule.features {
        if required
            .iter()
            .any(|(name, expected)| features.is_enabled(name) != *expected)
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OsRule;
    use std::collections::HashMap;

    fn rule(action: &str, os: Option<&str>, feature: Option<(&str, bool)>) -> Rule {
        Rule {
            action: action.into(),
            os: os.map(|n| OsRule {
                name: Some(n.into()),
                version: None,
                arch: None,
            }),
            features: feature.map(|(k, v)| HashMap::from([(k.to_string(), v)])),
        }
    }

    #[test]
    fn empty_rules_allow() {
        assert!(rules_allow(&[]));
    }

    #[test]
    fn allow_all_then_deny_other_os() {
        let other = if current_os_name() == "windows" { "linux" } else { "windows" };
        let rules = vec![rule("allow", None, None), rule("disallow", Some(other), None)];
        assert!(rules_allow(&rules));
    }

    #[test]
    fn deny_current_os() {
        let rules = vec![rule("allow", None, None), rule("disallow", Some(current_os_name()), None)];
        assert!(!rules_allow(&rules));
    }

    #[test]
    fn only_matching_os_allowed() {
        let rules = vec![rule("allow", Some(current_os_name()), None)];
        assert!(rules_allow(&rules));
        let other = if current_os_name() == "windows" { "linux" } else { "windows" };
        assert!(!rules_allow(&[rule("allow", Some(other), None)]));
    }

    #[test]
    fn feature_rules_follow_enabled_set() {
        let rules = vec![rule("allow", None, Some(("has_custom_resolution", true)))];
        assert!(!rules_allow(&rules));
        assert!(rules_allow_with(
            &rules,
            &RuleFeatures::none().with("has_custom_resolution")
        ));
    }

    #[test]
    fn os_version_rules_never_match() {
        let mut r = rule("allow", Some(current_os_name()), None);
        r.os.as_mut().unwrap().version = Some("^10\\.".into());
        assert!(!rules_allow(&[r]));
    }
}
