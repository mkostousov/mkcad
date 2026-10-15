//! Parameter store with dependency graph, cycle detection, and validation.
//!
//! Manages user-defined parameters (expressions), their dependencies, and dimension kinds.
//! Enforces naming rules, detects cycles, and supports dependent updates.

use crate::expr::EvalContext;
use crate::units::{DimensionKind, Quantity, UnitTable};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Parameter validation error.
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterError {
    /// Name violates naming rules (reason: syntax, reserved_unit, or duplicate).
    InvalidName {
        /// The invalid parameter name.
        name: String,
        /// Reason for invalidity.
        reason: String,
    },
    /// Expression contains a cycle (self or indirect).
    ExpressionCycle {
        /// The parameter name involved in the cycle.
        name: String,
        /// Chain of names forming the cycle.
        cycle_chain: Vec<String>,
    },
    /// Expression evaluation failed.
    ExpressionError {
        /// The parameter name.
        name: String,
        /// Error message.
        error: String,
    },
    /// Parameter in use cannot be deleted.
    InUse {
        /// The parameter name.
        name: String,
        /// Parameters that depend on this one.
        dependents: Vec<String>,
    },
    /// Unit mismatch in expression for this dimension.
    UnitMismatch {
        /// The parameter name.
        name: String,
        /// Expected dimension kind.
        expected: DimensionKind,
        /// Got dimension kind.
        got: DimensionKind,
    },
}

impl std::fmt::Display for ParameterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterError::InvalidName { name, reason } => {
                write!(f, "Invalid parameter name '{}': {}", name, reason)
            }
            ParameterError::ExpressionCycle { name, cycle_chain } => {
                write!(f, "Expression cycle in '{}': {}", name, cycle_chain.join(" -> "))
            }
            ParameterError::ExpressionError { name, error } => {
                write!(f, "Expression error in '{}': {}", name, error)
            }
            ParameterError::InUse { name, dependents } => {
                write!(f, "Parameter '{}' in use by: {}", name, dependents.join(", "))
            }
            ParameterError::UnitMismatch { name, expected, got } => {
                write!(f, "Unit mismatch in '{}': expected {:?}, got {:?}", name, expected, got)
            }
        }
    }
}

/// A single parameter definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name (unique, case-sensitive).
    pub name: String,
    /// Expression text as written (preserves units).
    pub expression: String,
    /// Expected dimension kind (length, angle, mass, unitless).
    pub dimension: DimensionKind,
    /// Cached evaluated value.
    #[serde(skip)]
    pub cached_value: Option<Quantity>,
}

impl Parameter {
    /// Creates a new parameter (without validation or evaluation).
    pub fn new(name: String, expression: String, dimension: DimensionKind) -> Self {
        Parameter { name, expression, dimension, cached_value: None }
    }
}

/// Serializable representation of parameter store for save/load.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterStoreSerialized {
    /// All parameters.
    pub parameters: Vec<Parameter>,
}

/// Parameter store with dependency tracking.
#[derive(Serialize, Deserialize)]
pub struct ParameterStore {
    /// Parameter definitions.
    #[serde(skip)]
    parameters: HashMap<String, Parameter>,
    /// Unit table for validation.
    #[serde(skip)]
    unit_table: UnitTable,
    /// Dependency graph: param -> set of params it depends on.
    #[serde(skip)]
    dependencies: HashMap<String, HashSet<String>>,
}

impl ParameterStore {
    /// Creates a new empty parameter store.
    pub fn new() -> Self {
        ParameterStore {
            parameters: HashMap::new(),
            unit_table: UnitTable::new(),
            dependencies: HashMap::new(),
        }
    }

    /// Validates a parameter name according to spec rules (F54).
    fn validate_name(&self, name: &str, check_duplicate: bool) -> Result<(), ParameterError> {
        // Must match [A-Za-z_][A-Za-z0-9_]*
        if name.is_empty() {
            return Err(ParameterError::InvalidName {
                name: name.to_string(),
                reason: "syntax: empty name".to_string(),
            });
        }

        let first_char = name.chars().next().unwrap();
        if !first_char.is_alphabetic() && first_char != '_' {
            return Err(ParameterError::InvalidName {
                name: name.to_string(),
                reason: "syntax: must start with letter or underscore".to_string(),
            });
        }

        for c in name.chars() {
            if !c.is_alphanumeric() && c != '_' {
                return Err(ParameterError::InvalidName {
                    name: name.to_string(),
                    reason: "syntax: contains invalid character".to_string(),
                });
            }
        }

        // Must not equal a reserved unit symbol
        if self.unit_table.is_reserved(name) {
            return Err(ParameterError::InvalidName {
                name: name.to_string(),
                reason: "reserved_unit".to_string(),
            });
        }

        // Must not be a reserved constant
        if name == "PI" || name == "E" {
            return Err(ParameterError::InvalidName {
                name: name.to_string(),
                reason: "reserved_constant".to_string(),
            });
        }

        // Check for duplicate if requested (for add_new)
        if check_duplicate && self.parameters.contains_key(name) {
            return Err(ParameterError::InvalidName {
                name: name.to_string(),
                reason: "duplicate".to_string(),
            });
        }

        Ok(())
    }

    /// Adds a new parameter; rejects if name already exists (EX-05).
    pub fn add(
        &mut self,
        name: String,
        expression: String,
        dimension: DimensionKind,
    ) -> Result<(), ParameterError> {
        // Validate name with duplicate check enabled
        self.validate_name(&name, true)?;
        // Delegate to common implementation
        self.set_or_update_impl(name, expression, dimension, true)
    }

    /// Sets or updates a parameter; allows overwriting existing (for internal use).
    pub fn set_or_update(
        &mut self,
        name: String,
        expression: String,
        dimension: DimensionKind,
    ) -> Result<(), ParameterError> {
        // Validate name but allow duplicates
        self.validate_name(&name, false)?;
        // Delegate to common implementation
        self.set_or_update_impl(name, expression, dimension, false)
    }

    /// Sets a parameter from user input, appending the display unit.
    /// E.g., set_from_user_input("Len", "10", "mm") → "10 mm", dimension Length
    pub fn set_from_user_input(
        &mut self,
        name: String,
        user_input: &str,
        display_unit: &str,
        dimension: DimensionKind,
    ) -> Result<(), ParameterError> {
        let expression = format!("{} {}", user_input, display_unit);
        self.set_or_update(name, expression, dimension)
    }

    /// Internal common implementation for add/update.
    fn set_or_update_impl(
        &mut self,
        name: String,
        expression: String,
        dimension: DimensionKind,
        _is_add: bool,
    ) -> Result<(), ParameterError> {
        // Check for self-reference (even if parameter doesn't exist yet in store)
        // by looking for the parameter name in the expression
        if self.expression_contains_identifier(&expression, &name) {
            return Err(ParameterError::ExpressionCycle {
                name: name.clone(),
                cycle_chain: vec![name.clone(), name.clone()],
            });
        }

        // Parse and evaluate expression
        let mut ctx = EvalContext::new();

        // Inject existing parameters into context
        for (pname, param) in &self.parameters {
            if pname != &name {
                if let Some(cached) = param.cached_value {
                    ctx.set_parameter(pname.clone(), cached);
                }
            }
        }

        let result = ctx.eval(&expression).map_err(|e| ParameterError::ExpressionError {
            name: name.clone(),
            error: e.to_string(),
        })?;

        // Check dimension compatibility
        if result.dimension != dimension {
            return Err(ParameterError::UnitMismatch {
                name: name.clone(),
                expected: dimension,
                got: result.dimension,
            });
        }

        // Detect cycles by finding dependencies in the expression
        let new_deps = self.extract_dependencies(&expression);

        // Check for indirect cycles using DFS
        if self.would_create_cycle(&name, &new_deps) {
            let cycle_chain = self.find_cycle_chain(&name, &new_deps);
            return Err(ParameterError::ExpressionCycle { name: name.clone(), cycle_chain });
        }

        // Update parameter and dependencies
        let mut param = Parameter::new(name.clone(), expression, dimension);
        param.cached_value = Some(result);

        self.parameters.insert(name.clone(), param);
        self.dependencies.insert(name.clone(), new_deps);

        // Auto-update dependents in topological order
        let _ = self.update_dependents_topological(&name);

        Ok(())
    }

    /// Deprecated: use add() or set_or_update() instead.
    #[deprecated(since = "0.1.0", note = "use add() or set_or_update()")]
    pub fn add_or_update(
        &mut self,
        name: String,
        expression: String,
        dimension: DimensionKind,
    ) -> Result<(), ParameterError> {
        self.set_or_update(name, expression, dimension)
    }

    /// Checks if an expression contains a specific identifier (parameter name).
    fn expression_contains_identifier(&self, expr: &str, target: &str) -> bool {
        let mut i = 0;
        let chars: Vec<char> = expr.chars().collect();

        while i < chars.len() {
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let mut ident = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    ident.push(chars[i]);
                    i += 1;
                }

                if ident == target {
                    return true;
                }
            } else {
                i += 1;
            }
        }

        false
    }

    /// Extracts parameter references from an expression string.
    fn extract_dependencies(&self, expr: &str) -> HashSet<String> {
        let mut deps = HashSet::new();

        // Simple heuristic: find identifiers that are parameter names
        let mut i = 0;
        let chars: Vec<char> = expr.chars().collect();

        while i < chars.len() {
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let mut ident = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    ident.push(chars[i]);
                    i += 1;
                }

                // Check if it's a known parameter (not a constant or function)
                if ident != "PI" && ident != "E" && ident != "max" && ident != "min" && ident != "abs"
                    && ident != "sqrt" && ident != "sin" && ident != "cos" && ident != "tan"
                    && ident != "sin_deg" && ident != "cos_deg"
                    && self.parameters.contains_key(&ident)
                {
                    deps.insert(ident);
                }
            } else {
                i += 1;
            }
        }

        deps
    }

    /// Checks if adding new_deps to param would create a cycle.
    fn would_create_cycle(&self, param: &str, new_deps: &HashSet<String>) -> bool {
        // DFS to check if any of param's dependencies depends on param
        let mut visited = HashSet::new();
        self.has_path_to(new_deps, param, &mut visited)
    }

    /// Checks if there's a path from the given set of nodes to target.
    fn has_path_to(&self, from: &HashSet<String>, target: &str, visited: &mut HashSet<String>) -> bool {
        for node in from {
            if visited.contains(node) {
                continue;
            }
            visited.insert(node.clone());

            if node == target {
                return true;
            }

            if let Some(deps) = self.dependencies.get(node) {
                if self.has_path_to(deps, target, visited) {
                    return true;
                }
            }
        }
        false
    }

    /// Finds a cycle chain starting from param.
    fn find_cycle_chain(&self, param: &str, new_deps: &HashSet<String>) -> Vec<String> {
        let mut path = vec![param.to_string()];
        let mut visited = HashSet::new();
        self.find_cycle_dfs(param, new_deps, &mut path, &mut visited);
        path
    }

    /// DFS helper to find a cycle path.
    fn find_cycle_dfs(
        &self,
        node: &str,
        next_deps: &HashSet<String>,
        path: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> bool {
        let deps_to_check = if path.len() == 1 {
            next_deps.clone()
        } else if let Some(deps) = self.dependencies.get(node) {
            deps.clone()
        } else {
            HashSet::new()
        };

        for dep in deps_to_check {
            if visited.contains(&dep) {
                continue;
            }
            visited.insert(dep.clone());

            if dep == path[0] {
                path.push(dep);
                return true;
            }

            path.push(dep.clone());
            if self.find_cycle_dfs(&dep, &HashSet::new(), path, visited) {
                return true;
            }
            path.pop();
        }

        false
    }

    /// Deletes a parameter if it's not in use.
    pub fn delete(&mut self, name: &str) -> Result<(), ParameterError> {
        // Find all parameters that depend on this one
        let dependents: Vec<String> =
            self.dependencies.iter().filter(|(_, deps)| deps.contains(name)).map(|(k, _)| k.clone()).collect();

        if !dependents.is_empty() {
            return Err(ParameterError::InUse { name: name.to_string(), dependents });
        }

        self.parameters.remove(name);
        self.dependencies.remove(name);

        Ok(())
    }

    /// Gets a parameter by name.
    pub fn get(&self, name: &str) -> Option<&Parameter> {
        self.parameters.get(name)
    }

    /// Gets all parameters.
    pub fn all(&self) -> Vec<&Parameter> {
        self.parameters.values().collect()
    }

    /// Updates dependent parameters after a value changes.
    /// Returns the list of updated parameters and any errors.
    pub fn update_dependents(&mut self, changed_param: &str) -> Result<Vec<String>, Vec<ParameterError>> {
        let mut updated = Vec::new();
        let mut errors = Vec::new();

        // Find all parameters that depend on changed_param (collect first to avoid borrow issues)
        let dependents: Vec<(String, String, DimensionKind)> = self
            .dependencies
            .iter()
            .filter(|(name, deps)| deps.contains(changed_param) && name.as_str() != changed_param)
            .filter_map(|(name, _)| {
                self.parameters.get(name).map(|p| (name.clone(), p.expression.clone(), p.dimension))
            })
            .collect();

        // Now update each dependent by re-evaluating with current parameters
        for (name, expr, dim) in dependents {
            // Build evaluation context with current parameter values
            let mut ctx = EvalContext::new();
            for (pname, param) in &self.parameters {
                if let Some(cached) = param.cached_value {
                    ctx.set_parameter(pname.clone(), cached);
                }
            }

            // Evaluate the expression
            match ctx.eval(&expr) {
                Ok(result) => {
                    if result.dimension == dim {
                        // Update the parameter's cached value
                        if let Some(param) = self.parameters.get_mut(&name) {
                            param.cached_value = Some(result);
                            updated.push(name);
                        }
                    } else {
                        errors.push(ParameterError::UnitMismatch {
                            name: name.clone(),
                            expected: dim,
                            got: result.dimension,
                        });
                    }
                }
                Err(e) => {
                    errors.push(ParameterError::ExpressionError { name: name.clone(), error: e.to_string() });
                }
            }
        }

        if errors.is_empty() {
            Ok(updated)
        } else {
            Err(errors)
        }
    }

    /// Updates dependent parameters in topological order after a change.
    fn update_dependents_topological(&mut self, changed_param: &str) -> Result<Vec<String>, Vec<ParameterError>> {
        let mut updated = Vec::new();
        let mut errors = Vec::new();

        // Find direct dependents
        let dependents: Vec<String> = self
            .dependencies
            .iter()
            .filter(|(name, deps)| deps.contains(changed_param) && name.as_str() != changed_param)
            .map(|(k, _)| k.clone())
            .collect();

        // Update each dependent and recursively update their dependents
        for dep_name in dependents {
            if let Some(param) = self.parameters.get(&dep_name) {
                let expr = param.expression.clone();
                let dim = param.dimension;

                // Build evaluation context with current values
                let mut ctx = EvalContext::new();
                for (pname, param) in &self.parameters {
                    if let Some(cached) = param.cached_value {
                        ctx.set_parameter(pname.clone(), cached);
                    }
                }

                // Evaluate the expression
                match ctx.eval(&expr) {
                    Ok(result) => {
                        if result.dimension == dim {
                            // Update the parameter's cached value
                            if let Some(param) = self.parameters.get_mut(&dep_name) {
                                param.cached_value = Some(result);
                            }
                            updated.push(dep_name.clone());
                            // Recursively update this parameter's dependents
                            match self.update_dependents_topological(&dep_name) {
                                Ok(recursive_updated) => updated.extend(recursive_updated),
                                Err(recursive_errors) => errors.extend(recursive_errors),
                            }
                        } else {
                            errors.push(ParameterError::UnitMismatch {
                                name: dep_name.clone(),
                                expected: dim,
                                got: result.dimension,
                            });
                        }
                    }
                    Err(e) => {
                        errors.push(ParameterError::ExpressionError {
                            name: dep_name.clone(),
                            error: e.to_string(),
                        });
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(updated)
        } else {
            Err(errors)
        }
    }

    /// Serializes the parameter store to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let serialized = ParameterStoreSerialized {
            parameters: self.parameters.values().cloned().collect(),
        };
        serde_json::to_string(&serialized)
    }

    /// Deserializes a parameter store from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let serialized: ParameterStoreSerialized = serde_json::from_str(json)?;
        let mut store = ParameterStore::new();
        for param in serialized.parameters {
            // Re-add parameters with re-evaluation
            let _ = store.set_or_update(param.name, param.expression, param.dimension);
        }
        Ok(store)
    }

    /// Returns the number of parameters.
    pub fn count(&self) -> usize {
        self.parameters.len()
    }

    /// Returns parameter dependencies.
    pub fn dependencies(&self) -> &HashMap<String, HashSet<String>> {
        &self.dependencies
    }
}

impl Default for ParameterStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_creation() {
        let param = Parameter::new("Len".to_string(), "40 mm".to_string(), DimensionKind::Length);
        assert_eq!(param.name, "Len");
        assert_eq!(param.expression, "40 mm");
        assert_eq!(param.dimension, DimensionKind::Length);
    }

    #[test]
    fn test_store_add_simple() {
        let mut store = ParameterStore::new();
        let result = store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length);
        assert!(result.is_ok());
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_store_validate_name_invalid_start() {
        let store = ParameterStore::new();
        let result = store.validate_name("1abc", false);
        assert!(matches!(result, Err(ParameterError::InvalidName { reason, .. }) if reason.contains("syntax")));
    }

    #[test]
    fn test_store_validate_name_reserved_unit() {
        let store = ParameterStore::new();
        let result = store.validate_name("mm", false);
        assert!(matches!(result, Err(ParameterError::InvalidName { reason, .. }) if reason == "reserved_unit"));
    }

    #[test]
    fn test_store_validate_name_reserved_constant() {
        let store = ParameterStore::new();
        let result = store.validate_name("PI", false);
        assert!(matches!(result, Err(ParameterError::InvalidName { reason, .. }) if reason == "reserved_constant"));
    }

    #[test]
    fn test_store_duplicate_name() {
        let mut store = ParameterStore::new();
        store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        let result = store.add_or_update("Len".to_string(), "50 mm".to_string(), DimensionKind::Length);
        // Update should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_store_self_cycle() {
        let mut store = ParameterStore::new();
        let result = store.add_or_update("Param".to_string(), "Param + 1 mm".to_string(), DimensionKind::Length);
        assert!(matches!(result, Err(ParameterError::ExpressionCycle { .. })));
    }

    #[test]
    fn test_store_indirect_cycle() {
        let mut store = ParameterStore::new();
        store.add_or_update("X".to_string(), "10 mm".to_string(), DimensionKind::Length).unwrap();
        store.add_or_update("Y".to_string(), "X + 20 mm".to_string(), DimensionKind::Length).unwrap();
        let result = store.add_or_update("X".to_string(), "Y + 30 mm".to_string(), DimensionKind::Length);
        assert!(matches!(result, Err(ParameterError::ExpressionCycle { .. })));
    }

    #[test]
    fn test_store_unit_mismatch() {
        let mut store = ParameterStore::new();
        let result = store.add_or_update("Len".to_string(), "40 kg".to_string(), DimensionKind::Length);
        assert!(matches!(result, Err(ParameterError::UnitMismatch { .. })));
    }

    #[test]
    fn test_store_get_parameter() {
        let mut store = ParameterStore::new();
        store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        let param = store.get("Len");
        assert!(param.is_some());
        assert_eq!(param.unwrap().name, "Len");
    }

    #[test]
    fn test_store_delete_unused() {
        let mut store = ParameterStore::new();
        store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        let result = store.delete("Len");
        assert!(result.is_ok());
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_store_delete_in_use() {
        let mut store = ParameterStore::new();
        store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        store.add_or_update("Half".to_string(), "Len / 2".to_string(), DimensionKind::Length).unwrap();
        let result = store.delete("Len");
        assert!(matches!(result, Err(ParameterError::InUse { .. })));
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn test_store_dependent_update() {
        let mut store = ParameterStore::new();
        store.add_or_update("Len".to_string(), "40 mm".to_string(), DimensionKind::Length).unwrap();
        store.add_or_update("MyW".to_string(), "Len / 2".to_string(), DimensionKind::Length).unwrap();

        // Update Len
        store.add_or_update("Len".to_string(), "100 mm".to_string(), DimensionKind::Length).unwrap();

        // Manually update dependents
        let _ = store.update_dependents("Len");

        // MyW should be 50 mm
        let my_w = store.get("MyW").unwrap();
        assert!(my_w.cached_value.is_some());
        assert_eq!(my_w.cached_value.unwrap().value, 50.0);
    }

    #[test]
    fn test_store_invalid_expression() {
        let mut store = ParameterStore::new();
        let result = store.add_or_update("Len".to_string(), "invalid( mm".to_string(), DimensionKind::Length);
        assert!(matches!(result, Err(ParameterError::ExpressionError { .. })));
    }

    #[test]
    fn test_store_unknown_identifier() {
        let mut store = ParameterStore::new();
        let result = store.add_or_update("Len".to_string(), "UnknownParam mm".to_string(), DimensionKind::Length);
        // UnknownParam is treated as a function call with missing paren, which might error
        // or be treated as an unknown identifier - depends on the parser
        assert!(result.is_err());
    }
}
