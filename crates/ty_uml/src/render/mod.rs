//! Multi-format UML diagram rendering.
//!
//! This module provides renderers for generating UML diagrams in various formats:
//! - Graphviz DOT
//! - `PlantUML`
//! - `MermaidJS`

mod config;
mod dot;
mod mermaid;
mod plantuml;

pub use config::{ColorScheme, DiagramDirection, RenderConfig};
pub use dot::DotRenderer;
pub use mermaid::MermaidRenderer;
pub use plantuml::PlantUmlRenderer;

use std::fmt;

use crate::model::UmlDiagram;

/// Supported output formats for UML diagrams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Graphviz DOT format (.dot, .gv)
    Dot,
    /// `PlantUML` format (.puml, .plantuml)
    PlantUml,
    /// `MermaidJS` format (.mmd)
    #[default]
    Mermaid,
}

impl OutputFormat {
    /// Get the primary file extension for this format.
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Dot => "dot",
            OutputFormat::PlantUml => "puml",
            OutputFormat::Mermaid => "mmd",
        }
    }

    /// Parse a format from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dot" | "gv" | "graphviz" => Some(OutputFormat::Dot),
            "puml" | "plantuml" => Some(OutputFormat::PlantUml),
            "mmd" | "mermaid" => Some(OutputFormat::Mermaid),
            _ => None,
        }
    }
}

/// Trait for UML diagram renderers.
pub trait UmlRenderer {
    /// Render the diagram to a formatter.
    fn render(&self, f: &mut fmt::Formatter<'_>, diagram: &UmlDiagram) -> fmt::Result;

    /// Get the file extension for this format.
    fn file_extension(&self) -> &'static str;
}

/// Wrapper for displaying a diagram in a specific format.
pub struct DisplayDiagram<'a, R: UmlRenderer> {
    diagram: &'a UmlDiagram,
    renderer: R,
}

impl<'a, R: UmlRenderer> DisplayDiagram<'a, R> {
    /// Create a new display wrapper.
    pub fn new(diagram: &'a UmlDiagram, renderer: R) -> Self {
        Self { diagram, renderer }
    }
}

impl<R: UmlRenderer> fmt::Display for DisplayDiagram<'_, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.renderer.render(f, self.diagram)
    }
}

/// Render a diagram to a string in the specified format.
pub fn render_to_string(
    diagram: &UmlDiagram,
    format: OutputFormat,
    config: &RenderConfig,
) -> String {
    match format {
        OutputFormat::Dot => format!("{}", DisplayDiagram::new(diagram, DotRenderer::new(config))),
        OutputFormat::PlantUml => {
            format!(
                "{}",
                DisplayDiagram::new(diagram, PlantUmlRenderer::new(config))
            )
        }
        OutputFormat::Mermaid => {
            format!(
                "{}",
                DisplayDiagram::new(diagram, MermaidRenderer::new(config))
            )
        }
    }
}
