//! Call graph structures for UML diagrams.

use super::{ClassId, UmlParameter, Visibility};

/// Unique identifier for a function (fully qualified name).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionId(pub String);

impl FunctionId {
    /// Create a function ID for a module-level function.
    pub fn module_function(module: &str, name: &str) -> Self {
        if module.is_empty() {
            Self(name.to_string())
        } else {
            Self(format!("{module}.{name}"))
        }
    }

    /// Create a function ID for a class method.
    pub fn method(module: &str, class: &str, name: &str) -> Self {
        if module.is_empty() {
            Self(format!("{class}.{name}"))
        } else {
            Self(format!("{module}.{class}.{name}"))
        }
    }

    /// Get the fully qualified name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Represents a function or method in the call graph.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlFunction {
    /// Unique identifier.
    pub id: FunctionId,
    /// Function name.
    pub name: String,
    /// Module path where the function is defined.
    pub module_path: String,
    /// Containing class (if this is a method).
    pub containing_class: Option<ClassId>,
    /// Function parameters.
    pub parameters: Vec<UmlParameter>,
    /// Return type representation.
    pub return_type: Option<String>,
    /// Visibility (for methods).
    pub visibility: Visibility,
    /// Whether this is async.
    pub is_async: bool,
    /// Whether this is a generator.
    pub is_generator: bool,
}

impl UmlFunction {
    /// Create a new module-level function.
    pub fn new_function(id: FunctionId, name: impl Into<String>, module_path: String) -> Self {
        Self {
            id,
            name: name.into(),
            module_path,
            containing_class: None,
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_generator: false,
        }
    }

    /// Create a new method.
    pub fn new_method(
        id: FunctionId,
        name: impl Into<String>,
        module_path: String,
        class_id: ClassId,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            module_path,
            containing_class: Some(class_id),
            parameters: Vec::new(),
            return_type: None,
            visibility: Visibility::Public,
            is_async: false,
            is_generator: false,
        }
    }

    /// Check if this is a method (belongs to a class).
    pub fn is_method(&self) -> bool {
        self.containing_class.is_some()
    }
}

/// Represents a function call edge in the call graph.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionCall {
    /// The calling function.
    pub caller: FunctionId,
    /// The called function.
    pub callee: FunctionId,
    /// Location of the call (line number or range).
    pub call_site: Option<String>,
    /// Whether this is a direct call or through an alias.
    pub is_direct: bool,
}

impl FunctionCall {
    /// Create a new function call edge.
    pub fn new(caller: FunctionId, callee: FunctionId) -> Self {
        Self {
            caller,
            callee,
            call_site: None,
            is_direct: true,
        }
    }

    /// Add call site information.
    #[must_use]
    pub fn at_line(mut self, line: u32) -> Self {
        self.call_site = Some(format!("line {line}"));
        self
    }

    /// Mark as indirect call.
    #[must_use]
    pub fn indirect(mut self) -> Self {
        self.is_direct = false;
        self
    }
}
