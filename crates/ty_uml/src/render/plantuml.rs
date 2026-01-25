//! `PlantUML` format renderer.

use std::fmt;

use crate::model::{ClassKind, RelationshipKind, UmlClass, UmlDiagram, Visibility};

use super::{RenderConfig, UmlRenderer};

/// Renderer for `PlantUML` format.
pub struct PlantUmlRenderer<'a> {
    config: &'a RenderConfig,
}

impl<'a> PlantUmlRenderer<'a> {
    /// Create a new `PlantUML` renderer.
    pub fn new(config: &'a RenderConfig) -> Self {
        Self { config }
    }

    /// Sanitize a class ID for `PlantUML` (dots become underscores).
    fn sanitize_id(id: &str) -> String {
        id.replace(['.', '-'], "_")
    }

    /// Get the `PlantUML` keyword for a class kind.
    fn class_keyword(kind: ClassKind) -> &'static str {
        match kind {
            ClassKind::Protocol => "interface",
            ClassKind::Abstract => "abstract class",
            ClassKind::Enum => "enum",
            ClassKind::Regular
            | ClassKind::TypedDict
            | ClassKind::Dataclass
            | ClassKind::NamedTuple
            | ClassKind::Exception => "class",
        }
    }

    /// Render a class definition.
    fn render_class(&self, f: &mut fmt::Formatter<'_>, class: &UmlClass) -> fmt::Result {
        let keyword = Self::class_keyword(class.kind);
        let id = Self::sanitize_id(&class.id.0);

        // Class declaration with display name
        writeln!(f, r#"{keyword} "{}" as {id} {{"#, class.name)?;

        // Stereotype for special class kinds
        match class.kind {
            ClassKind::TypedDict => writeln!(f, "  <<TypedDict>>")?,
            ClassKind::Dataclass => writeln!(f, "  <<dataclass>>")?,
            ClassKind::NamedTuple => writeln!(f, "  <<namedtuple>>")?,
            ClassKind::Exception => writeln!(f, "  <<exception>>")?,
            _ => {}
        }

        // Type parameters
        if !class.type_parameters.is_empty() {
            let params = class.type_parameters.join(", ");
            writeln!(f, "  <<Generic: {params}>>")?;
        }

        // Attributes
        for attr in &class.attributes {
            let vis = if self.config.show_visibility {
                Self::visibility_char(attr.visibility)
            } else {
                ' '
            };
            let type_str = if self.config.show_types && !attr.type_repr.is_empty() {
                format!(": {}", self.config.truncate_type(&attr.type_repr))
            } else {
                String::new()
            };
            let modifiers = Self::attribute_modifiers(attr);
            writeln!(f, "  {modifiers}{vis}{}{type_str}", attr.name)?;
        }

        // Methods
        for method in &class.methods {
            let vis = if self.config.show_visibility {
                Self::visibility_char(method.visibility)
            } else {
                ' '
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
            writeln!(f, "  {modifiers}{vis}{}(){ret}", method.name)?;
        }

        writeln!(f, "}}")?;
        Ok(())
    }

    /// Get the visibility character.
    fn visibility_char(vis: Visibility) -> char {
        match vis {
            Visibility::Public => '+',
            Visibility::Protected => '#',
            Visibility::Private => '-',
        }
    }

    /// Get attribute modifiers for `PlantUML`.
    fn attribute_modifiers(attr: &crate::model::UmlAttribute) -> &'static str {
        if attr.is_class_var {
            "{static} "
        } else if attr.is_final {
            "{readonly} "
        } else {
            ""
        }
    }

    /// Get method modifiers for `PlantUML`.
    fn method_modifiers(method: &crate::model::UmlMethod) -> &'static str {
        if method.is_static {
            "{static} "
        } else if method.is_abstract {
            "{abstract} "
        } else if method.is_classmethod {
            "{classifier} "
        } else {
            ""
        }
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

        let arrow = match rel.kind {
            RelationshipKind::Inheritance => "--|>",
            RelationshipKind::Realization => "..|>",
            RelationshipKind::Composition => "*--",
            RelationshipKind::Aggregation => "o--",
            RelationshipKind::Association => "-->",
            RelationshipKind::Dependency => "..>",
        };

        write!(f, "{source} {arrow} {target}")?;
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
        writeln!(f, r#"class "{name}" as {id} <<external>> {{"#)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl UmlRenderer for PlantUmlRenderer<'_> {
    fn render(&self, f: &mut fmt::Formatter<'_>, diagram: &UmlDiagram) -> fmt::Result {
        writeln!(f, "@startuml")?;

        if let Some(title) = &self.config.title {
            writeln!(f, "title {title}")?;
        }

        // Styling
        writeln!(f, "skinparam classAttributeIconSize 0")?;
        writeln!(f, "skinparam classFontStyle bold")?;

        writeln!(f)?;

        // Group by module if configured
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
                    writeln!(f, "package \"{module}\" {{")?;
                    for class in classes {
                        self.render_class(f, class)?;
                        writeln!(f)?;
                    }
                    writeln!(f, "}}")?;
                } else {
                    for class in classes {
                        self.render_class(f, class)?;
                        writeln!(f)?;
                    }
                }
            }
        } else {
            for class in diagram.classes.values() {
                self.render_class(f, class)?;
                writeln!(f)?;
            }
        }

        // External references
        if self.config.show_external_refs {
            writeln!(f, "' External references")?;
            for class_id in &diagram.external_class_refs {
                Self::render_external_ref(f, &class_id.0)?;
            }
            writeln!(f)?;
        }

        // Relationships
        writeln!(f, "' Relationships")?;
        for rel in &diagram.relationships {
            self.render_relationship(f, rel, diagram)?;
        }

        writeln!(f, "@enduml")?;

        Ok(())
    }

    fn file_extension(&self) -> &'static str {
        "puml"
    }
}
