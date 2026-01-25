//! Configuration for UML extraction.

/// Configuration options for UML extraction.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ExtractionConfig {
    /// Include private members (names starting with single underscore).
    pub include_private: bool,
    /// Include dunder methods (__init__, __str__, etc.).
    pub include_dunder: bool,
    /// Include inherited members in class diagrams.
    pub include_inherited: bool,
    /// Maximum depth for following relationships.
    pub max_depth: Option<usize>,
    /// Filter to specific module prefixes.
    pub module_filter: Option<Vec<String>>,
    /// Extract class diagrams.
    pub extract_classes: bool,
    /// Extract module dependencies.
    pub extract_modules: bool,
    /// Extract function call graphs.
    pub extract_calls: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            include_private: false,
            include_dunder: true,
            include_inherited: false,
            max_depth: Some(3),
            module_filter: None,
            extract_classes: true,
            extract_modules: true,
            extract_calls: true,
        }
    }
}

impl ExtractionConfig {
    /// Create a configuration that only extracts class diagrams.
    pub fn classes_only() -> Self {
        Self {
            extract_classes: true,
            extract_modules: false,
            extract_calls: false,
            ..Self::default()
        }
    }

    /// Create a configuration that only extracts module dependencies.
    pub fn modules_only() -> Self {
        Self {
            extract_classes: false,
            extract_modules: true,
            extract_calls: false,
            ..Self::default()
        }
    }

    /// Create a configuration that only extracts call graphs.
    pub fn calls_only() -> Self {
        Self {
            extract_classes: false,
            extract_modules: false,
            extract_calls: true,
            ..Self::default()
        }
    }

    /// Include all members (private and dunder).
    #[must_use]
    pub fn include_all_members(mut self) -> Self {
        self.include_private = true;
        self.include_dunder = true;
        self
    }

    /// Set the maximum relationship depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Remove depth limit.
    #[must_use]
    pub fn unlimited_depth(mut self) -> Self {
        self.max_depth = None;
        self
    }

    /// Filter to specific module prefixes.
    #[must_use]
    pub fn with_module_filter(mut self, prefixes: Vec<String>) -> Self {
        self.module_filter = Some(prefixes);
        self
    }

    /// Check if a member name should be included based on configuration.
    pub fn should_include_member(&self, name: &str) -> bool {
        let is_dunder = name.starts_with("__") && name.ends_with("__");
        let is_private = name.starts_with('_') && !is_dunder;

        if is_dunder && !self.include_dunder {
            return false;
        }
        if is_private && !self.include_private {
            return false;
        }
        true
    }
}
