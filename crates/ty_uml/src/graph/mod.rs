//! Graph database storage traits for UML diagrams.
//!
//! This module provides trait definitions for storing UML diagrams in graph databases.
//! The traits are designed to be generic enough to support various graph database backends
//! like Neo4j, `SQLite` with graph extensions, or in-memory graph structures.

mod traits;

pub use traits::{GraphStorage, IncrementalGraphStorage};
