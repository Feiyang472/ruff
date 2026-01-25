#![warn(
    clippy::disallowed_methods,
    reason = "Prefer System trait methods over std methods in ty crates"
)]

pub mod extract;
pub mod graph;
pub mod model;
pub mod render;

pub use extract::{ExtractionConfig, extract_diagram};
pub use model::{
    ClassId, ClassKind, FunctionCall, FunctionId, RelationshipKind, UmlAttribute, UmlClass,
    UmlDiagram, UmlFunction, UmlMethod, UmlModule, UmlRelationship, Visibility,
};
pub use render::{OutputFormat, RenderConfig, UmlRenderer, render_to_string};
