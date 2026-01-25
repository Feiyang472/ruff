use ruff_db::files::File;
use ruff_python_ast::{self as ast, Stmt};
use ty_module_resolver::file_to_module;
use ty_python_semantic::types::list_members::all_members;
use ty_python_semantic::{Db, HasType, SemanticModel};

use crate::model::{
    ClassId, ClassKind, RelationshipKind, UmlAttribute, UmlClass, UmlDiagram, UmlMethod,
    UmlRelationship, Visibility,
};

use super::ExtractionConfig;

/// Extracts class information from Python source files.
pub struct ClassExtractor<'a, 'db> {
    db: &'db dyn Db,
    file: File,
    config: &'a ExtractionConfig,
    module_path: String,
}

impl<'a, 'db> ClassExtractor<'a, 'db> {
    /// Create a new class extractor.
    pub fn new(db: &'db dyn Db, file: File, config: &'a ExtractionConfig) -> Self {
        let module_path = file_to_module(db, file)
            .map(|m| m.name(db).to_string())
            .unwrap_or_default();

        Self {
            db,
            file,
            config,
            module_path,
        }
    }

    /// Extract classes from a module AST.
    pub fn extract_from_module(&self, diagram: &mut UmlDiagram, module: &ast::ModModule) {
        if !self.config.extract_classes {
            return;
        }

        for stmt in &module.body {
            self.extract_from_stmt(diagram, stmt);
        }
    }

    /// Extract classes from a statement.
    fn extract_from_stmt(&self, diagram: &mut UmlDiagram, stmt: &Stmt) {
        match stmt {
            Stmt::ClassDef(class_def) => {
                self.extract_class(diagram, class_def);
            }
            // Handle nested classes in functions if needed
            Stmt::FunctionDef(func_def) => {
                for stmt in &func_def.body {
                    self.extract_from_stmt(diagram, stmt);
                }
            }
            Stmt::If(if_stmt) => {
                for stmt in &if_stmt.body {
                    self.extract_from_stmt(diagram, stmt);
                }
                for clause in &if_stmt.elif_else_clauses {
                    for stmt in &clause.body {
                        self.extract_from_stmt(diagram, stmt);
                    }
                }
            }
            Stmt::With(with_stmt) => {
                for stmt in &with_stmt.body {
                    self.extract_from_stmt(diagram, stmt);
                }
            }
            Stmt::Try(try_stmt) => {
                for stmt in &try_stmt.body {
                    self.extract_from_stmt(diagram, stmt);
                }
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(inner) = handler;
                    for stmt in &inner.body {
                        self.extract_from_stmt(diagram, stmt);
                    }
                }
                for stmt in &try_stmt.orelse {
                    self.extract_from_stmt(diagram, stmt);
                }
                for stmt in &try_stmt.finalbody {
                    self.extract_from_stmt(diagram, stmt);
                }
            }
            _ => {}
        }
    }

    /// Extract a single class definition.
    fn extract_class(&self, diagram: &mut UmlDiagram, class_def: &ast::StmtClassDef) {
        let class_name = class_def.name.id.clone();
        let class_id = ClassId::new(&self.module_path, class_name.as_str());

        // Skip if already extracted
        if diagram.has_class(&class_id) {
            return;
        }

        // Determine class kind from decorators and bases
        let kind = Self::determine_class_kind(class_def);

        // Create the UML class
        let mut uml_class = UmlClass::new(class_id.clone(), class_name, self.module_path.clone());
        uml_class.kind = kind;

        // Extract type parameters if present
        if let Some(type_params) = &class_def.type_params {
            uml_class.type_parameters = type_params
                .type_params
                .iter()
                .map(|tp| match tp {
                    ast::TypeParam::TypeVar(tv) => tv.name.to_string(),
                    ast::TypeParam::ParamSpec(ps) => ps.name.to_string(),
                    ast::TypeParam::TypeVarTuple(tvt) => tvt.name.to_string(),
                })
                .collect();
        }

        // Extract members using the semantic model
        self.extract_class_members(&mut uml_class, class_def);

        // Add class to diagram
        diagram.add_class(uml_class);

        // Extract inheritance relationships from base classes
        self.extract_inheritance(diagram, &class_id, class_def);

        // Extract nested classes
        for stmt in &class_def.body {
            if let Stmt::ClassDef(nested_class) = stmt {
                self.extract_class(diagram, nested_class);
            }
        }
    }

    /// Determine the kind of class from decorators and base classes.
    fn determine_class_kind(class_def: &ast::StmtClassDef) -> ClassKind {
        // Check decorators
        for decorator in &class_def.decorator_list {
            if let Some(name) = Self::get_decorator_name(&decorator.expression) {
                match name.as_str() {
                    "dataclass" | "dataclasses.dataclass" => return ClassKind::Dataclass,
                    _ => {}
                }
            }
        }

        // Check base classes
        for base in class_def.bases() {
            if let Some(name) = Self::get_base_class_name(base) {
                match name.as_str() {
                    "Protocol" | "typing.Protocol" | "typing_extensions.Protocol" => {
                        return ClassKind::Protocol;
                    }
                    "TypedDict" | "typing.TypedDict" | "typing_extensions.TypedDict" => {
                        return ClassKind::TypedDict;
                    }
                    "Enum" | "enum.Enum" | "IntEnum" | "enum.IntEnum" | "StrEnum"
                    | "enum.StrEnum" | "Flag" | "enum.Flag" => {
                        return ClassKind::Enum;
                    }
                    "ABC" | "abc.ABC" => return ClassKind::Abstract,
                    "Exception" | "BaseException" => return ClassKind::Exception,
                    "NamedTuple" | "typing.NamedTuple" => return ClassKind::NamedTuple,
                    _ => {}
                }
            }
        }

        // Check for abstract methods
        for stmt in &class_def.body {
            if let Stmt::FunctionDef(func) = stmt {
                for decorator in &func.decorator_list {
                    if let Some(name) = Self::get_decorator_name(&decorator.expression) {
                        if name == "abstractmethod" || name == "abc.abstractmethod" {
                            return ClassKind::Abstract;
                        }
                    }
                }
            }
        }

        ClassKind::Regular
    }

    /// Get the name of a decorator expression.
    fn get_decorator_name(expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                let value = Self::get_decorator_name(&attr.value)?;
                Some(format!("{}.{}", value, attr.attr))
            }
            ast::Expr::Call(call) => Self::get_decorator_name(&call.func),
            _ => None,
        }
    }

    /// Get the name of a base class expression.
    fn get_base_class_name(expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => Some(name.id.to_string()),
            ast::Expr::Attribute(attr) => {
                let value = Self::get_base_class_name(&attr.value)?;
                Some(format!("{}.{}", value, attr.attr))
            }
            ast::Expr::Subscript(sub) => Self::get_base_class_name(&sub.value),
            _ => None,
        }
    }

    /// Extract members (attributes and methods) from a class.
    fn extract_class_members(&self, uml_class: &mut UmlClass, class_def: &ast::StmtClassDef) {
        let model = SemanticModel::new(self.db, self.file);

        // Try to get the type of the class and use all_members
        if let Some(class_type) = class_def.inferred_type(&model) {
            // Get instance type for member enumeration
            let members = all_members(self.db, class_type);

            for member in members {
                let name_str = member.name.as_str();

                // Apply filtering
                if !self.config.should_include_member(name_str) {
                    continue;
                }

                let visibility = Visibility::from_name(name_str);
                let type_repr = format!("{}", member.ty.display(self.db));

                // Determine if this is a method or attribute based on type display
                if Self::is_method_type(&type_repr) {
                    uml_class.methods.push(UmlMethod {
                        name: member.name.to_string(),
                        parameters: Vec::new(), // Would need signature parsing
                        return_type: Self::extract_return_type(&type_repr),
                        visibility,
                        is_static: false,
                        is_classmethod: false,
                        is_property: type_repr.contains("property"),
                        is_abstract: false,
                    });
                } else {
                    uml_class.attributes.push(UmlAttribute {
                        name: member.name.to_string(),
                        type_repr,
                        visibility,
                        is_class_var: false,
                        is_final: false,
                        is_read_only: false,
                    });
                }
            }
        }

        // Also extract from AST for more detailed information
        self.extract_members_from_ast(uml_class, class_def);
    }

    /// Check if a type representation indicates a method/callable.
    fn is_method_type(type_repr: &str) -> bool {
        type_repr.starts_with("def ")
            || type_repr.starts_with("Callable")
            || type_repr.contains("-> ")
    }

    /// Extract return type from a function type representation.
    fn extract_return_type(type_repr: &str) -> Option<String> {
        type_repr
            .rfind(" -> ")
            .map(|pos| type_repr[pos + 4..].to_string())
    }

    /// Extract members directly from AST for additional details.
    fn extract_members_from_ast(&self, uml_class: &mut UmlClass, class_def: &ast::StmtClassDef) {
        for stmt in &class_def.body {
            match stmt {
                Stmt::FunctionDef(func) => {
                    self.extract_method_from_ast(uml_class, func, func.is_async);
                }
                Stmt::AnnAssign(ann_assign) => {
                    self.extract_annotated_attribute(uml_class, ann_assign);
                }
                Stmt::Assign(assign) => {
                    self.extract_assigned_attribute(uml_class, assign);
                }
                _ => {}
            }
        }
    }

    /// Extract method information from AST.
    fn extract_method_from_ast(
        &self,
        uml_class: &mut UmlClass,
        func: &ast::StmtFunctionDef,
        _is_async: bool,
    ) {
        let name_str = func.name.id.as_str();

        if !self.config.should_include_member(name_str) {
            return;
        }

        // Skip if already added from type system
        if uml_class.methods.iter().any(|m| m.name == name_str) {
            return;
        }

        let visibility = Visibility::from_name(name_str);

        // Check decorators
        let mut is_static = false;
        let mut is_classmethod = false;
        let mut is_property = false;
        let mut is_abstract = false;

        for decorator in &func.decorator_list {
            if let Some(dec_name) = Self::get_decorator_name(&decorator.expression) {
                match dec_name.as_str() {
                    "staticmethod" => is_static = true,
                    "classmethod" => is_classmethod = true,
                    "property" => is_property = true,
                    "abstractmethod" | "abc.abstractmethod" => is_abstract = true,
                    _ => {}
                }
            }
        }

        // Extract return type annotation
        let return_type = func.returns.as_ref().map(|ret| Self::expr_to_string(ret));

        uml_class.methods.push(UmlMethod {
            name: name_str.to_string(),
            parameters: Vec::new(), // Could extract from func.parameters
            return_type,
            visibility,
            is_static,
            is_classmethod,
            is_property,
            is_abstract,
        });
    }

    /// Extract annotated class attribute.
    fn extract_annotated_attribute(
        &self,
        uml_class: &mut UmlClass,
        ann_assign: &ast::StmtAnnAssign,
    ) {
        let name_str = match &*ann_assign.target {
            ast::Expr::Name(name) => name.id.as_str(),
            _ => return,
        };

        if !self.config.should_include_member(name_str) {
            return;
        }

        // Skip if already added
        if uml_class.attributes.iter().any(|a| a.name == name_str) {
            return;
        }

        let type_repr = Self::expr_to_string(&ann_assign.annotation);
        let visibility = Visibility::from_name(name_str);

        // Check for ClassVar and Final
        let is_class_var = type_repr.starts_with("ClassVar");
        let is_final = type_repr.starts_with("Final");

        uml_class.attributes.push(UmlAttribute {
            name: name_str.to_string(),
            type_repr,
            visibility,
            is_class_var,
            is_final,
            is_read_only: false,
        });
    }

    /// Extract assigned class attribute (without annotation).
    fn extract_assigned_attribute(&self, uml_class: &mut UmlClass, assign: &ast::StmtAssign) {
        for target in &assign.targets {
            if let ast::Expr::Name(name) = target {
                let name_str = name.id.as_str();
                if !self.config.should_include_member(name_str) {
                    continue;
                }

                // Skip if already added
                if uml_class.attributes.iter().any(|a| a.name == name_str) {
                    continue;
                }

                let visibility = Visibility::from_name(name_str);

                uml_class.attributes.push(UmlAttribute {
                    name: name_str.to_string(),
                    type_repr: String::new(), // No type annotation
                    visibility,
                    is_class_var: true, // Class-level assignment
                    is_final: false,
                    is_read_only: false,
                });
            }
        }
    }

    /// Convert an expression to a string representation.
    fn expr_to_string(expr: &ast::Expr) -> String {
        match expr {
            ast::Expr::Name(name) => name.id.to_string(),
            ast::Expr::Attribute(attr) => {
                format!("{}.{}", Self::expr_to_string(&attr.value), attr.attr)
            }
            ast::Expr::Subscript(sub) => {
                format!(
                    "{}[{}]",
                    Self::expr_to_string(&sub.value),
                    Self::expr_to_string(&sub.slice)
                )
            }
            ast::Expr::Tuple(tuple) => {
                let elements: Vec<_> = tuple.elts.iter().map(Self::expr_to_string).collect();
                elements.join(", ")
            }
            ast::Expr::List(list) => {
                let elements: Vec<_> = list.elts.iter().map(Self::expr_to_string).collect();
                format!("[{}]", elements.join(", "))
            }
            ast::Expr::BinOp(binop) => {
                let op = match binop.op {
                    ast::Operator::BitOr => " | ",
                    _ => " ? ",
                };
                format!(
                    "{}{}{}",
                    Self::expr_to_string(&binop.left),
                    op,
                    Self::expr_to_string(&binop.right)
                )
            }
            ast::Expr::NoneLiteral(_) => "None".to_string(),
            ast::Expr::EllipsisLiteral(_) => "...".to_string(),
            ast::Expr::StringLiteral(s) => format!("\"{}\"", s.value),
            ast::Expr::NumberLiteral(n) => format!("{:?}", n.value),
            _ => "...".to_string(),
        }
    }

    /// Extract inheritance relationships from base classes.
    fn extract_inheritance(
        &self,
        diagram: &mut UmlDiagram,
        class_id: &ClassId,
        class_def: &ast::StmtClassDef,
    ) {
        for base in class_def.bases() {
            if let Some(base_name) = Self::get_base_class_name(base) {
                // Skip object
                if base_name == "object" {
                    continue;
                }

                // Determine if this is a protocol implementation or regular inheritance
                let kind = if base_name.contains("Protocol") {
                    RelationshipKind::Realization
                } else {
                    RelationshipKind::Inheritance
                };

                // Create target class ID
                // If the base is from a different module, we may not have the full path
                let target_id = if base_name.contains('.') {
                    ClassId(base_name.clone())
                } else {
                    // Assume same module for unqualified names
                    ClassId::new(&self.module_path, &base_name)
                };

                // Add relationship
                diagram.add_relationship(UmlRelationship::new(
                    class_id.clone(),
                    target_id.clone(),
                    kind,
                ));

                // Track external reference if not in diagram
                if !diagram.has_class(&target_id) {
                    diagram.add_external_class_ref(target_id);
                }
            }
        }
    }
}
