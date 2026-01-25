//! `MermaidJS` format renderer.

use std::fmt;

use crate::model::{RelationshipKind, UmlClass, UmlDiagram, Visibility};

use super::{DiagramDirection, RenderConfig, UmlRenderer};

/// Renderer for `MermaidJS` format.
pub struct MermaidRenderer<'a> {
    config: &'a RenderConfig,
}

impl<'a> MermaidRenderer<'a> {
    /// Create a new Mermaid renderer.
    pub fn new(config: &'a RenderConfig) -> Self {
        Self { config }
    }

    /// Sanitize a class ID for Mermaid (dots and hyphens become underscores).
    fn sanitize_id(id: &str) -> String {
        id.replace(['.', '-'], "_")
    }

    /// Escape special characters for Mermaid labels.
    fn escape_label(s: &str) -> String {
        s.replace('"', "'")
            .replace(['<', '>'], "~")
            .replace('{', "(")
            .replace('}', ")")
    }

    /// Get the Mermaid direction string.
    fn direction_str(dir: DiagramDirection) -> &'static str {
        match dir {
            DiagramDirection::TopToBottom => "TB",
            DiagramDirection::LeftToRight => "LR",
            DiagramDirection::BottomToTop => "BT",
            DiagramDirection::RightToLeft => "RL",
        }
    }

    /// Render a class definition.
    fn render_class(&self, f: &mut fmt::Formatter<'_>, class: &UmlClass) -> fmt::Result {
        let id = Self::sanitize_id(&class.id.0);

        writeln!(f, "    class {id} {{")?;

        // Annotation for class kind
        if let Some(stereo) = class.kind.stereotype() {
            writeln!(f, "        <<{stereo}>>")?;
        }

        // Type parameters as annotation
        if !class.type_parameters.is_empty() {
            let params = class.type_parameters.join(", ");
            writeln!(f, "        <<Generic: {params}>>")?;
        }

        // Attributes
        for attr in &class.attributes {
            let vis = if self.config.show_visibility {
                Self::visibility_symbol(attr.visibility)
            } else {
                ""
            };
            let type_str = if self.config.show_types && !attr.type_repr.is_empty() {
                let truncated = self.config.truncate_type(&attr.type_repr);
                format!(" {}", Self::escape_label(&truncated))
            } else {
                String::new()
            };
            writeln!(f, "        {vis}{}{type_str}", attr.name)?;
        }

        // Methods
        for method in &class.methods {
            let vis = if self.config.show_visibility {
                Self::visibility_symbol(method.visibility)
            } else {
                ""
            };
            let ret = if self.config.show_types {
                method
                    .return_type
                    .as_ref()
                    .map(|r| {
                        let truncated = self.config.truncate_type(r);
                        format!(" {}", Self::escape_label(&truncated))
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let prefix = Self::method_prefix(method);
            let suffix = Self::method_suffix(method);
            writeln!(f, "        {prefix}{vis}{}(){suffix}{ret}", method.name)?;
        }

        writeln!(f, "    }}")?;
        Ok(())
    }

    /// Get the visibility symbol for Mermaid.
    fn visibility_symbol(vis: Visibility) -> &'static str {
        match vis {
            Visibility::Public => "+",
            Visibility::Protected => "#",
            Visibility::Private => "-",
        }
    }

    /// Get method prefix for Mermaid (static marker).
    fn method_prefix(method: &crate::model::UmlMethod) -> &'static str {
        if method.is_static { "$" } else { "" }
    }

    /// Get method suffix for Mermaid (abstract marker).
    fn method_suffix(method: &crate::model::UmlMethod) -> &'static str {
        if method.is_abstract { "*" } else { "" }
    }

    /// Render a relationship.
    fn render_relationship(
        &self,
        f: &mut fmt::Formatter<'_>,
        rel: &crate::model::UmlRelationship,
        diagram: &UmlDiagram,
    ) -> fmt::Result {
        // Skip if target is external and we're not showing external refs
        if !self.config.show_external_refs && !diagram.has_class(&rel.target) {
            return Ok(());
        }

        let source = Self::sanitize_id(&rel.source.0);
        let target = Self::sanitize_id(&rel.target.0);

        // Mermaid puts the parent/target on the left for inheritance
        let arrow = match rel.kind {
            RelationshipKind::Inheritance => "<|--",
            RelationshipKind::Realization => "<|..",
            RelationshipKind::Composition => "*--",
            RelationshipKind::Aggregation => "o--",
            RelationshipKind::Association => "-->",
            RelationshipKind::Dependency => "..>",
        };

        // For inheritance/realization, target is the parent (on left)
        match rel.kind {
            RelationshipKind::Inheritance | RelationshipKind::Realization => {
                write!(f, "    {target} {arrow} {source}")?;
            }
            _ => {
                write!(f, "    {source} {arrow} {target}")?;
            }
        }

        if let Some(label) = &rel.label {
            write!(f, " : {label}")?;
        }
        writeln!(f)?;

        Ok(())
    }

    /// Render an external class reference.
    fn render_external_ref(f: &mut fmt::Formatter<'_>, class_id: &str) -> fmt::Result {
        let id = Self::sanitize_id(class_id);
        let name = class_id.rsplit('.').next().unwrap_or(class_id);
        writeln!(f, "    class {id} {{")?;
        writeln!(f, "        <<external>>")?;
        writeln!(f, "    }}")?;
        writeln!(f, "    note for {id} \"{name}\"")?;
        Ok(())
    }
}

impl UmlRenderer for MermaidRenderer<'_> {
    fn render(&self, f: &mut fmt::Formatter<'_>, diagram: &UmlDiagram) -> fmt::Result {
        writeln!(f, "classDiagram")?;
        let dir = Self::direction_str(self.config.direction);
        writeln!(f, "    direction {dir}")?;

        if let Some(title) = &self.config.title {
            writeln!(f, "    %%{{title: {title}}}%%")?;
        }

        writeln!(f)?;

        // Note: Mermaid doesn't support true subgraph/package grouping for class diagrams
        // We render all classes at the top level
        for class in diagram.classes.values() {
            self.render_class(f, class)?;
            writeln!(f)?;
        }

        // External references
        if self.config.show_external_refs {
            writeln!(f, "    %% External references")?;
            for class_id in &diagram.external_class_refs {
                Self::render_external_ref(f, &class_id.0)?;
            }
            writeln!(f)?;
        }

        // Relationships
        writeln!(f, "    %% Relationships")?;
        for rel in &diagram.relationships {
            self.render_relationship(f, rel, diagram)?;
        }

        Ok(())
    }

    fn file_extension(&self) -> &'static str {
        "mmd"
    }
}
