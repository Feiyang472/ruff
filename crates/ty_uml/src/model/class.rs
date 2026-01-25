//! Class-related UML model structures.

/// Unique identifier for a class (fully qualified name).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClassId(pub String);

impl ClassId {
    /// Create a new class ID from module path and class name.
    pub fn new(module: &str, name: &str) -> Self {
        if module.is_empty() {
            Self(name.to_string())
        } else {
            Self(format!("{module}.{name}"))
        }
    }

    /// Get the fully qualified name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClassId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Visibility modifier for class members, inferred from Python naming conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Visibility {
    /// Public member (no underscore prefix).
    #[default]
    Public,
    /// Protected member (single underscore prefix).
    Protected,
    /// Private member (double underscore prefix, name mangled).
    Private,
}

impl Visibility {
    /// Infer visibility from Python naming conventions.
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("__") && !name.ends_with("__") {
            Visibility::Private
        } else if name.starts_with('_') {
            Visibility::Protected
        } else {
            Visibility::Public
        }
    }

    /// Get the UML visibility symbol.
    pub fn symbol(self) -> char {
        match self {
            Visibility::Public => '+',
            Visibility::Protected => '#',
            Visibility::Private => '-',
        }
    }
}

/// Classification of Python classes for UML representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ClassKind {
    /// Regular class.
    #[default]
    Regular,
    /// Abstract class (has ABC in bases or abstract methods).
    Abstract,
    /// Protocol (structural typing interface).
    Protocol,
    /// `TypedDict` (dictionary with typed keys).
    TypedDict,
    /// Enum class.
    Enum,
    /// Dataclass or dataclass-like.
    Dataclass,
    /// `NamedTuple`.
    NamedTuple,
    /// Exception class.
    Exception,
}

impl ClassKind {
    /// Get the UML stereotype string for this class kind.
    pub fn stereotype(self) -> Option<&'static str> {
        match self {
            ClassKind::Regular => None,
            ClassKind::Abstract => Some("abstract"),
            ClassKind::Protocol => Some("interface"),
            ClassKind::TypedDict => Some("TypedDict"),
            ClassKind::Enum => Some("enumeration"),
            ClassKind::Dataclass => Some("dataclass"),
            ClassKind::NamedTuple => Some("namedtuple"),
            ClassKind::Exception => Some("exception"),
        }
    }
}

/// A class attribute in the UML model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlAttribute {
    /// Attribute name.
    pub name: String,
    /// String representation of the type.
    pub type_repr: String,
    /// Visibility modifier.
    pub visibility: Visibility,
    /// Whether this is a class variable (`ClassVar`).
    pub is_class_var: bool,
    /// Whether this is marked Final.
    pub is_final: bool,
    /// Whether this is read-only (for `TypedDict` `ReadOnly` fields).
    pub is_read_only: bool,
}

/// Kind of method parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ParameterKind {
    /// Positional-only parameter (before /).
    PositionalOnly,
    /// Positional or keyword parameter (default).
    #[default]
    PositionalOrKeyword,
    /// Variadic positional (*args).
    VarPositional,
    /// Keyword-only parameter (after *).
    KeywordOnly,
    /// Variadic keyword (**kwargs).
    VarKeyword,
}

/// A method parameter in the UML model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlParameter {
    /// Parameter name.
    pub name: String,
    /// String representation of the type annotation.
    pub type_repr: Option<String>,
    /// Whether the parameter has a default value.
    pub has_default: bool,
    /// Kind of parameter.
    pub kind: ParameterKind,
}

/// A class method in the UML model.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::struct_excessive_bools)]
pub struct UmlMethod {
    /// Method name.
    pub name: String,
    /// Method parameters.
    pub parameters: Vec<UmlParameter>,
    /// String representation of the return type.
    pub return_type: Option<String>,
    /// Visibility modifier.
    pub visibility: Visibility,
    /// Whether this is a static method.
    pub is_static: bool,
    /// Whether this is a class method.
    pub is_classmethod: bool,
    /// Whether this is a property.
    pub is_property: bool,
    /// Whether this is an abstract method.
    pub is_abstract: bool,
}

/// Represents a Python class in UML.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UmlClass {
    /// Unique identifier (fully qualified name).
    pub id: ClassId,
    /// Simple class name.
    pub name: String,
    /// Module path where the class is defined.
    pub module_path: String,
    /// Class attributes.
    pub attributes: Vec<UmlAttribute>,
    /// Class methods.
    pub methods: Vec<UmlMethod>,
    /// Classification of the class.
    pub kind: ClassKind,
    /// Generic type parameters (e.g., T, K, V).
    pub type_parameters: Vec<String>,
}

impl UmlClass {
    /// Create a new UML class with the given ID and name.
    pub fn new(id: ClassId, name: impl Into<String>, module_path: String) -> Self {
        Self {
            id,
            name: name.into(),
            module_path,
            attributes: Vec::new(),
            methods: Vec::new(),
            kind: ClassKind::Regular,
            type_parameters: Vec::new(),
        }
    }
}
