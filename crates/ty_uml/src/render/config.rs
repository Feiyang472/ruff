//! Configuration for UML rendering.

/// Configuration options for UML rendering.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct RenderConfig {
    /// Title for the diagram.
    pub title: Option<String>,
    /// Whether to show visibility symbols (+, -, #).
    pub show_visibility: bool,
    /// Whether to show type annotations.
    pub show_types: bool,
    /// Whether to show method parameters.
    pub show_parameters: bool,
    /// Whether to group classes by package/module.
    pub group_by_module: bool,
    /// Direction for diagram layout.
    pub direction: DiagramDirection,
    /// Color scheme.
    pub color_scheme: ColorScheme,
    /// Whether to include external references (classes not in the diagram).
    pub show_external_refs: bool,
    /// Maximum width for type annotations (truncate if longer).
    pub max_type_width: Option<usize>,
}

/// Direction for diagram layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramDirection {
    /// Top to bottom (default for class diagrams).
    #[default]
    TopToBottom,
    /// Left to right.
    LeftToRight,
    /// Bottom to top.
    BottomToTop,
    /// Right to left.
    RightToLeft,
}

impl DiagramDirection {
    /// Get the DOT rankdir value.
    pub fn dot_rankdir(self) -> &'static str {
        match self {
            DiagramDirection::TopToBottom => "TB",
            DiagramDirection::LeftToRight => "LR",
            DiagramDirection::BottomToTop => "BT",
            DiagramDirection::RightToLeft => "RL",
        }
    }
}

/// Color scheme for diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    /// Default colors.
    #[default]
    Default,
    /// Monochrome (black and white).
    Monochrome,
    /// Custom colors per class kind.
    ByClassKind,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            title: None,
            show_visibility: true,
            show_types: true,
            show_parameters: true,
            group_by_module: true,
            direction: DiagramDirection::default(),
            color_scheme: ColorScheme::default(),
            show_external_refs: false,
            max_type_width: Some(40),
        }
    }
}

impl RenderConfig {
    /// Create a minimal configuration (no types, no visibility).
    pub fn minimal() -> Self {
        Self {
            show_visibility: false,
            show_types: false,
            show_parameters: false,
            ..Self::default()
        }
    }

    /// Set the diagram title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the diagram direction.
    #[must_use]
    pub fn with_direction(mut self, direction: DiagramDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Enable or disable visibility symbols.
    #[must_use]
    pub fn with_visibility(mut self, show: bool) -> Self {
        self.show_visibility = show;
        self
    }

    /// Enable or disable type annotations.
    #[must_use]
    pub fn with_types(mut self, show: bool) -> Self {
        self.show_types = show;
        self
    }

    /// Show external references.
    #[must_use]
    pub fn with_external_refs(mut self, show: bool) -> Self {
        self.show_external_refs = show;
        self
    }

    /// Truncate a type string if it exceeds the maximum width.
    pub fn truncate_type(&self, type_repr: &str) -> String {
        match self.max_type_width {
            Some(max) if type_repr.len() > max => {
                format!("{}...", &type_repr[..max.saturating_sub(3)])
            }
            _ => type_repr.to_string(),
        }
    }
}
