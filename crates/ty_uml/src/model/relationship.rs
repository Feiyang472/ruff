//! Relationship types for UML diagrams.

use super::ClassId;

/// The kind of relationship between UML elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RelationshipKind {
    /// Inheritance (is-a) - solid line with hollow triangle.
    /// Represents class inheritance in Python.
    Inheritance,
    /// Realization (implements) - dashed line with hollow triangle.
    /// Represents protocol implementation in Python.
    Realization,
    /// Composition (strong has-a) - solid line with filled diamond.
    /// The contained object cannot exist without the container.
    Composition,
    /// Aggregation (weak has-a) - solid line with hollow diamond.
    /// The contained object can exist independently.
    Aggregation,
    /// Association (uses) - solid line with arrow.
    /// General relationship where one class uses another.
    Association,
    /// Dependency (depends-on) - dashed line with arrow.
    /// One class depends on another (e.g., uses in method signature).
    Dependency,
}

impl RelationshipKind {
    /// Get a human-readable description of this relationship kind.
    pub fn description(self) -> &'static str {
        match self {
            RelationshipKind::Inheritance => "inherits from",
            RelationshipKind::Realization => "implements",
            RelationshipKind::Composition => "contains",
            RelationshipKind::Aggregation => "has",
            RelationshipKind::Association => "uses",
            RelationshipKind::Dependency => "depends on",
        }
    }
}

/// A relationship between two UML elements.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlRelationship {
    /// Source class identifier.
    pub source: ClassId,
    /// Target class identifier.
    pub target: ClassId,
    /// Kind of relationship.
    pub kind: RelationshipKind,
    /// Optional label (e.g., role name).
    pub label: Option<String>,
    /// Source-side multiplicity (e.g., "1", "0..*").
    pub source_multiplicity: Option<String>,
    /// Target-side multiplicity.
    pub target_multiplicity: Option<String>,
}

impl UmlRelationship {
    /// Create a new relationship between two classes.
    pub fn new(source: ClassId, target: ClassId, kind: RelationshipKind) -> Self {
        Self {
            source,
            target,
            kind,
            label: None,
            source_multiplicity: None,
            target_multiplicity: None,
        }
    }

    /// Create an inheritance relationship.
    pub fn inheritance(child: ClassId, parent: ClassId) -> Self {
        Self::new(child, parent, RelationshipKind::Inheritance)
    }

    /// Create a realization (protocol implementation) relationship.
    pub fn realization(implementor: ClassId, protocol: ClassId) -> Self {
        Self::new(implementor, protocol, RelationshipKind::Realization)
    }

    /// Add a label to this relationship.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}
