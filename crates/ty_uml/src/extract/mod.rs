//! Extract UML elements from ty's type system.
//!
//! This module provides functionality to extract UML class diagrams, module
//! dependencies, and call graphs from Python source files analyzed by ty.

mod class_extractor;
mod config;

pub use class_extractor::ClassExtractor;
pub use config::ExtractionConfig;

use ruff_db::files::File;
use ruff_db::parsed::parsed_module;
use ty_python_semantic::Db;

use crate::model::UmlDiagram;

/// Extract a UML diagram from a file.
///
/// This function analyzes the given Python file and extracts class definitions,
/// inheritance relationships, and member information into a UML diagram.
pub fn extract_diagram(db: &dyn Db, file: File, config: &ExtractionConfig) -> UmlDiagram {
    let mut diagram = UmlDiagram::new();
    let extractor = ClassExtractor::new(db, file, config);

    let parsed = parsed_module(db, file).load(db);
    extractor.extract_from_module(&mut diagram, parsed.syntax());

    diagram
}

/// Extract UML diagrams from multiple files.
pub fn extract_diagrams(
    db: &dyn Db,
    files: impl IntoIterator<Item = File>,
    config: &ExtractionConfig,
) -> UmlDiagram {
    let mut diagram = UmlDiagram::new();

    for file in files {
        let file_diagram = extract_diagram(db, file, config);
        diagram.merge(file_diagram);
    }

    diagram
}
