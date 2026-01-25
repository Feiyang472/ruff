//! Graphviz DOT format renderer.

use std::fmt;

use crate::model::{ClassKind, RelationshipKind, UmlClass, UmlDiagram};

use super::{RenderConfig, UmlRenderer};

/// Renderer for Graphviz DOT format.
pub struct DotRenderer<'a> {
    config: &'a RenderConfig,
}

impl<'a> DotRenderer<'a> {
    /// Create a new DOT renderer.
    pub fn new(config: &'a RenderConfig) -> Self {
        Self { config }
    }

    /// Escape a string for DOT HTML labels.
    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    /// Get the node ID for a class (sanitized for DOT).
    fn node_id(class_id: &str) -> String {
        class_id.replace(['.', '-'], "_")
    }

    /// Get the background color for a class kind.
    fn class_color(&self, kind: ClassKind) -> &'static str {
        match self.config.color_scheme {
            super::ColorScheme::Monochrome => "white",
            super::ColorScheme::Default | super::ColorScheme::ByClassKind => match kind {
                ClassKind::Regular => "lightblue",
                ClassKind::Abstract => "lightyellow",
                ClassKind::Protocol => "lightgreen",
                ClassKind::TypedDict => "lightcoral",
                ClassKind::Enum => "plum",
                ClassKind::Dataclass => "lightcyan",
                ClassKind::NamedTuple => "wheat",
                ClassKind::Exception => "lightsalmon",
            },
        }
    }

    /// Render a class node.
    fn render_class(&self, f: &mut fmt::Formatter<'_>, class: &UmlClass) -> fmt::Result {
        let node_id = Self::node_id(&class.id.0);
        let color = self.class_color(class.kind);
        let stereotype = class.kind.stereotype();

        writeln!(f, r#"  "{node_id}" ["#)?;
        writeln!(f, r#"    label=<"#)?;
        writeln!(
            f,
            r#"      <TABLE BORDER="0" CELLBORDER="1" CELLSPACING="0" CELLPADDING="4">"#
        )?;

        // Class name header
        write!(
            f,
            r#"        <TR><TD BGCOLOR="{color}"><B>{}</B>"#,
            Self::escape_html(class.name.as_str())
        )?;
        if let Some(stereo) = stereotype {
            write!(
                f,
                r#"<BR/><FONT POINT-SIZE="10">&lt;&lt;{stereo}&gt;&gt;</FONT>"#
            )?;
        }
        if !class.type_parameters.is_empty() {
            write!(
                f,
                r#"<BR/><FONT POINT-SIZE="9">&lt;{}&gt;</FONT>"#,
                class.type_parameters.join(", ")
            )?;
        }
        writeln!(f, r#"</TD></TR>"#)?;

        // Attributes section
        writeln!(f, r#"        <TR><TD ALIGN="LEFT">"#)?;
        if class.attributes.is_empty() {
            writeln!(
                f,
                r#"          <FONT COLOR="gray">(no attributes)</FONT><BR/>"#
            )?;
        } else {
            for attr in &class.attributes {
                let vis = if self.config.show_visibility {
                    format!("{} ", attr.visibility.symbol())
                } else {
                    String::new()
                };
                let type_str = if self.config.show_types && !attr.type_repr.is_empty() {
                    format!(": {}", self.config.truncate_type(&attr.type_repr))
                } else {
                    String::new()
                };
                writeln!(
                    f,
                    r#"          {}{}{}<BR/>"#,
                    vis,
                    Self::escape_html(attr.name.as_str()),
                    Self::escape_html(&type_str)
                )?;
            }
        }
        writeln!(f, r#"        </TD></TR>"#)?;

        // Methods section
        writeln!(f, r#"        <TR><TD ALIGN="LEFT">"#)?;
        if class.methods.is_empty() {
            writeln!(
                f,
                r#"          <FONT COLOR="gray">(no methods)</FONT><BR/>"#
            )?;
        } else {
            for method in &class.methods {
                let vis = if self.config.show_visibility {
                    format!("{} ", method.visibility.symbol())
                } else {
                    String::new()
                };
                let ret = if self.config.show_types {
                    method
                        .return_type
                        .as_ref()
                        .map(|r| format!(": {}", self.config.truncate_type(r)))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let modifiers = Self::method_modifiers(method);
                writeln!(
                    f,
                    r#"          {}{}{}{}<BR/>"#,
                    modifiers,
                    vis,
                    Self::escape_html(method.name.as_str()),
                    Self::escape_html(&format!("(){ret}"))
                )?;
            }
        }
        writeln!(f, r#"        </TD></TR>"#)?;

        writeln!(f, r#"      </TABLE>>"#)?;
        writeln!(f, r#"    shape=none"#)?;
        writeln!(f, r#"  ];"#)?;

        Ok(())
    }

    /// Get method modifier prefix for display.
    fn method_modifiers(method: &crate::model::UmlMethod) -> &'static str {
        if method.is_static {
            "<U>static</U> "
        } else if method.is_classmethod {
            "<U>classmethod</U> "
        } else if method.is_abstract {
            "<I>abstract</I> "
        } else if method.is_property {
            "<I>property</I> "
        } else {
            ""
        }
    }

    /// Render a relationship edge.
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

        let source = Self::node_id(&rel.source.0);
        let target = Self::node_id(&rel.target.0);

        let (arrowhead, style, dir) = match rel.kind {
            RelationshipKind::Inheritance => ("empty", "solid", "back"),
            RelationshipKind::Realization => ("empty", "dashed", "back"),
            RelationshipKind::Composition => ("diamond", "solid", "back"),
            RelationshipKind::Aggregation => ("odiamond", "solid", "back"),
            RelationshipKind::Association => ("vee", "solid", "forward"),
            RelationshipKind::Dependency => ("vee", "dashed", "forward"),
        };

        write!(f, r#"  "{source}" -> "{target}" "#)?;
        writeln!(f, r#"[arrowhead={arrowhead}, style={style}, dir={dir}];"#)?;

        Ok(())
    }

    /// Render an external class reference (placeholder node).
    fn render_external_ref(f: &mut fmt::Formatter<'_>, class_id: &str) -> fmt::Result {
        let node_id = Self::node_id(class_id);
        let name = class_id.rsplit('.').next().unwrap_or(class_id);

        writeln!(f, r#"  "{node_id}" ["#)?;
        writeln!(
            f,
            r#"    label=<{}<BR/><FONT POINT-SIZE="9">(external)</FONT>>"#,
            Self::escape_html(name)
        )?;
        writeln!(f, r#"    style=dashed"#)?;
        writeln!(f, r#"    color=gray"#)?;
        writeln!(f, r#"  ];"#)?;

        Ok(())
    }
}

impl UmlRenderer for DotRenderer<'_> {
    fn render(&self, f: &mut fmt::Formatter<'_>, diagram: &UmlDiagram) -> fmt::Result {
        writeln!(f, "digraph UML {{")?;
        writeln!(f, "  rankdir={};", self.config.direction.dot_rankdir())?;
        writeln!(f, r#"  node [fontname="Helvetica", fontsize=12];"#)?;
        writeln!(f, r#"  edge [fontname="Helvetica", fontsize=10];"#)?;

        if let Some(title) = &self.config.title {
            writeln!(f, r#"  labelloc="t";"#)?;
            writeln!(f, r#"  label="{title}";"#)?;
        }

        writeln!(f)?;

        // Render classes grouped by module if configured
        if self.config.group_by_module {
            let mut modules: std::collections::HashMap<&str, Vec<&UmlClass>> =
                std::collections::HashMap::new();
            for class in diagram.classes.values() {
                modules
                    .entry(class.module_path.as_str())
                    .or_default()
                    .push(class);
            }

            for (module, classes) in modules {
                if !module.is_empty() {
                    let subgraph_name = Self::node_id(module);
                    writeln!(f, "  subgraph cluster_{subgraph_name} {{")?;
                    writeln!(f, r#"    label="{module}";"#)?;
                    writeln!(f, r#"    style=dashed;"#)?;
                    writeln!(f, r#"    color=gray;"#)?;
                    for class in classes {
                        self.render_class(f, class)?;
                    }
                    writeln!(f, "  }}")?;
                } else {
                    for class in classes {
                        self.render_class(f, class)?;
                    }
                }
            }
        } else {
            for class in diagram.classes.values() {
                self.render_class(f, class)?;
            }
        }

        // Render external references if configured
        if self.config.show_external_refs {
            writeln!(f)?;
            writeln!(f, "  // External references")?;
            for class_id in &diagram.external_class_refs {
                Self::render_external_ref(f, &class_id.0)?;
            }
        }

        writeln!(f)?;
        writeln!(f, "  // Relationships")?;
        for rel in &diagram.relationships {
            self.render_relationship(f, rel, diagram)?;
        }

        writeln!(f, "}}")?;

        Ok(())
    }

    fn file_extension(&self) -> &'static str {
        "dot"
    }
}
