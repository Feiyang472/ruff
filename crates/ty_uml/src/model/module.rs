//! Module-related UML model structures.

use super::ClassId;

/// Represents a Python module in UML.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlModule {
    /// Module name (e.g., `my_package.submodule`).
    pub name: String,
    /// Classes defined in this module.
    pub classes: Vec<ClassId>,
    /// Submodule names.
    pub submodules: Vec<String>,
    /// Whether this is a package (has __init__.py).
    pub is_package: bool,
}

impl UmlModule {
    /// Create a new module with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            classes: Vec::new(),
            submodules: Vec::new(),
            is_package: false,
        }
    }

    /// Mark this module as a package.
    #[must_use]
    pub fn as_package(mut self) -> Self {
        self.is_package = true;
        self
    }

    /// Add a class to this module.
    pub fn add_class(&mut self, class_id: ClassId) {
        self.classes.push(class_id);
    }

    /// Add a submodule to this module.
    pub fn add_submodule(&mut self, submodule: impl Into<String>) {
        self.submodules.push(submodule.into());
    }
}

/// A dependency between two modules.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ModuleDependency {
    /// The module that imports.
    pub importer: String,
    /// The module being imported.
    pub imported: String,
    /// The specific names imported (if `from x import y`).
    pub imported_names: Vec<String>,
    /// Whether this is a relative import.
    pub is_relative: bool,
}

impl ModuleDependency {
    /// Create a new module dependency.
    pub fn new(importer: impl Into<String>, imported: impl Into<String>) -> Self {
        Self {
            importer: importer.into(),
            imported: imported.into(),
            imported_names: Vec::new(),
            is_relative: false,
        }
    }

    /// Add imported names (for `from x import y, z`).
    #[must_use]
    pub fn with_names(mut self, names: Vec<String>) -> Self {
        self.imported_names = names;
        self
    }

    /// Mark as a relative import.
    #[must_use]
    pub fn relative(mut self) -> Self {
        self.is_relative = true;
        self
    }
}
