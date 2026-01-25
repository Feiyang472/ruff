//! UML diagram container.

use indexmap::IndexMap;
use rustc_hash::FxHashSet;

use super::{
    ClassId, FunctionCall, FunctionId, ModuleDependency, UmlClass, UmlFunction, UmlModule,
    UmlRelationship,
};

/// A complete UML diagram containing all extracted elements.
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlDiagram {
    /// All classes in the diagram, keyed by their ID.
    pub classes: IndexMap<ClassId, UmlClass>,
    /// All relationships between classes.
    pub relationships: Vec<UmlRelationship>,
    /// Module structure.
    pub modules: Vec<UmlModule>,
    /// Module dependencies.
    pub module_dependencies: Vec<ModuleDependency>,
    /// All functions in the call graph.
    pub functions: IndexMap<FunctionId, UmlFunction>,
    /// Function call edges.
    pub calls: Vec<FunctionCall>,
    /// Set of external class references (not extracted, just referenced).
    pub external_class_refs: FxHashSet<ClassId>,
    /// Set of external function references.
    pub external_function_refs: FxHashSet<FunctionId>,
}

impl UmlDiagram {
    /// Create a new empty diagram.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a class to the diagram.
    pub fn add_class(&mut self, class: UmlClass) {
        self.classes.insert(class.id.clone(), class);
    }

    /// Add a relationship to the diagram.
    pub fn add_relationship(&mut self, relationship: UmlRelationship) {
        self.relationships.push(relationship);
    }

    /// Add a module to the diagram.
    pub fn add_module(&mut self, module: UmlModule) {
        self.modules.push(module);
    }

    /// Add a module dependency.
    pub fn add_module_dependency(&mut self, dependency: ModuleDependency) {
        self.module_dependencies.push(dependency);
    }

    /// Add a function to the diagram.
    pub fn add_function(&mut self, function: UmlFunction) {
        self.functions.insert(function.id.clone(), function);
    }

    /// Add a function call edge.
    pub fn add_call(&mut self, call: FunctionCall) {
        self.calls.push(call);
    }

    /// Mark a class as externally referenced.
    pub fn add_external_class_ref(&mut self, class_id: ClassId) {
        self.external_class_refs.insert(class_id);
    }

    /// Mark a function as externally referenced.
    pub fn add_external_function_ref(&mut self, function_id: FunctionId) {
        self.external_function_refs.insert(function_id);
    }

    /// Check if a class is in the diagram.
    pub fn has_class(&self, class_id: &ClassId) -> bool {
        self.classes.contains_key(class_id)
    }

    /// Check if a function is in the diagram.
    pub fn has_function(&self, function_id: &FunctionId) -> bool {
        self.functions.contains_key(function_id)
    }

    /// Get a class by ID.
    pub fn get_class(&self, class_id: &ClassId) -> Option<&UmlClass> {
        self.classes.get(class_id)
    }

    /// Get a function by ID.
    pub fn get_function(&self, function_id: &FunctionId) -> Option<&UmlFunction> {
        self.functions.get(function_id)
    }

    /// Merge another diagram into this one.
    pub fn merge(&mut self, other: UmlDiagram) {
        self.classes.extend(other.classes);
        self.relationships.extend(other.relationships);
        self.modules.extend(other.modules);
        self.module_dependencies.extend(other.module_dependencies);
        self.functions.extend(other.functions);
        self.calls.extend(other.calls);
        self.external_class_refs.extend(other.external_class_refs);
        self.external_function_refs
            .extend(other.external_function_refs);
    }

    /// Get statistics about the diagram.
    pub fn stats(&self) -> DiagramStats {
        DiagramStats {
            class_count: self.classes.len(),
            relationship_count: self.relationships.len(),
            module_count: self.modules.len(),
            function_count: self.functions.len(),
            call_count: self.calls.len(),
        }
    }

    /// Check if the diagram is empty.
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty() && self.modules.is_empty() && self.functions.is_empty()
    }
}

/// Statistics about a UML diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct DiagramStats {
    /// Number of classes.
    pub class_count: usize,
    /// Number of relationships.
    pub relationship_count: usize,
    /// Number of modules.
    pub module_count: usize,
    /// Number of functions.
    pub function_count: usize,
    /// Number of function calls.
    pub call_count: usize,
}
