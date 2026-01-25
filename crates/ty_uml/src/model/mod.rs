//! Intermediate UML representation.
//!
//! This module provides data structures that represent UML diagram elements
//! in a format-agnostic way. These structures are lifetime-free (using owned
//! data) to allow them to outlive the Salsa database and be serialized.

mod call_graph;
mod class;
mod diagram;
mod module;
mod relationship;

pub use call_graph::{FunctionCall, FunctionId, UmlFunction};
pub use class::{
    ClassId, ClassKind, ParameterKind, UmlAttribute, UmlClass, UmlMethod, UmlParameter, Visibility,
};
pub use diagram::UmlDiagram;
pub use module::{ModuleDependency, UmlModule};
pub use relationship::{RelationshipKind, UmlRelationship};
