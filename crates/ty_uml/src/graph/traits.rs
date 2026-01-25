//! Graph storage trait definitions.

use crate::model::{
    ClassId, FunctionCall, FunctionId, ModuleDependency, UmlClass, UmlDiagram, UmlFunction,
    UmlModule, UmlRelationship,
};

/// Trait for graph storage backends.
///
/// This trait defines the interface for storing UML diagram elements in a graph database.
/// Implementations can target various backends like Neo4j, `SQLite`, or in-memory structures.
///
/// # Type Parameters
///
/// Each implementation defines its own node and edge ID types, as well as error types.
/// This allows backends to use their native ID representations.
///
/// # Example
///
/// ```ignore
/// struct InMemoryGraph {
///     nodes: HashMap<usize, NodeData>,
///     edges: Vec<(usize, usize, EdgeData)>,
///     next_id: usize,
/// }
///
/// impl GraphStorage for InMemoryGraph {
///     type Error = Infallible;
///     type NodeId = usize;
///     type EdgeId = usize;
///
///     fn store_class(&mut self, class: &UmlClass) -> Result<Self::NodeId, Self::Error> {
///         let id = self.next_id;
///         self.next_id += 1;
///         self.nodes.insert(id, NodeData::Class(class.clone()));
///         Ok(id)
///     }
///     // ... other methods
/// }
/// ```
pub trait GraphStorage {
    /// Error type for storage operations.
    type Error;
    /// Type used to identify nodes in the graph.
    type NodeId;
    /// Type used to identify edges in the graph.
    type EdgeId;

    /// Store a class as a node in the graph.
    ///
    /// Returns the node ID assigned to the class.
    fn store_class(&mut self, class: &UmlClass) -> Result<Self::NodeId, Self::Error>;

    /// Store a function as a node in the graph.
    ///
    /// Returns the node ID assigned to the function.
    fn store_function(&mut self, function: &UmlFunction) -> Result<Self::NodeId, Self::Error>;

    /// Store a module as a node in the graph.
    ///
    /// Returns the node ID assigned to the module.
    fn store_module(&mut self, module: &UmlModule) -> Result<Self::NodeId, Self::Error>;

    /// Store a class relationship as an edge in the graph.
    ///
    /// The source and target node IDs should correspond to previously stored classes.
    fn store_class_relationship(
        &mut self,
        rel: &UmlRelationship,
        source_node: Self::NodeId,
        target_node: Self::NodeId,
    ) -> Result<Self::EdgeId, Self::Error>;

    /// Store a function call as an edge in the graph.
    fn store_function_call(
        &mut self,
        call: &FunctionCall,
        caller_node: Self::NodeId,
        callee_node: Self::NodeId,
    ) -> Result<Self::EdgeId, Self::Error>;

    /// Store a module dependency as an edge in the graph.
    fn store_module_dependency(
        &mut self,
        dep: &ModuleDependency,
        importer_node: Self::NodeId,
        imported_node: Self::NodeId,
    ) -> Result<Self::EdgeId, Self::Error>;

    /// Store an entire diagram in the graph.
    ///
    /// This is a convenience method that stores all elements of a diagram.
    /// The default implementation stores classes first, then relationships.
    fn store_diagram(&mut self, diagram: &UmlDiagram) -> Result<(), Self::Error>
    where
        Self::NodeId: Clone,
    {
        use std::collections::HashMap;

        // Store all classes and track their node IDs
        let mut class_nodes: HashMap<ClassId, Self::NodeId> = HashMap::new();
        for (class_id, class) in &diagram.classes {
            let node_id = self.store_class(class)?;
            class_nodes.insert(class_id.clone(), node_id);
        }

        // Store all functions and track their node IDs
        let mut function_nodes: HashMap<FunctionId, Self::NodeId> = HashMap::new();
        for (function_id, function) in &diagram.functions {
            let node_id = self.store_function(function)?;
            function_nodes.insert(function_id.clone(), node_id);
        }

        // Store all modules and track their node IDs
        let mut module_nodes: HashMap<String, Self::NodeId> = HashMap::new();
        for module in &diagram.modules {
            let node_id = self.store_module(module)?;
            module_nodes.insert(module.name.clone(), node_id);
        }

        // Store class relationships
        for rel in &diagram.relationships {
            if let (Some(source), Some(target)) =
                (class_nodes.get(&rel.source), class_nodes.get(&rel.target))
            {
                self.store_class_relationship(rel, source.clone(), target.clone())?;
            }
        }

        // Store function calls
        for call in &diagram.calls {
            if let (Some(caller), Some(callee)) = (
                function_nodes.get(&call.caller),
                function_nodes.get(&call.callee),
            ) {
                self.store_function_call(call, caller.clone(), callee.clone())?;
            }
        }

        // Store module dependencies
        for dep in &diagram.module_dependencies {
            if let (Some(importer), Some(imported)) = (
                module_nodes.get(&dep.importer),
                module_nodes.get(&dep.imported),
            ) {
                self.store_module_dependency(dep, importer.clone(), imported.clone())?;
            }
        }

        Ok(())
    }

    /// Query a class by its ID.
    ///
    /// Returns `None` if the class is not found.
    fn get_class(&self, class_id: &ClassId) -> Result<Option<UmlClass>, Self::Error>;

    /// Query a function by its ID.
    fn get_function(&self, function_id: &FunctionId) -> Result<Option<UmlFunction>, Self::Error>;

    /// Query a module by its name.
    fn get_module(&self, module_name: &str) -> Result<Option<UmlModule>, Self::Error>;

    /// Get all classes in the graph.
    fn all_classes(&self) -> Result<Vec<UmlClass>, Self::Error>;

    /// Get all relationships for a class.
    fn class_relationships(&self, class_id: &ClassId) -> Result<Vec<UmlRelationship>, Self::Error>;
}

/// Extension trait for incremental graph updates.
///
/// This trait is useful when integrating with incremental build systems like Salsa,
/// where only changed elements need to be updated in the graph.
pub trait IncrementalGraphStorage: GraphStorage {
    /// Update a class if it has changed.
    ///
    /// If the class already exists, update it in place.
    /// If it doesn't exist, add it as a new node.
    fn upsert_class(&mut self, class: &UmlClass) -> Result<Self::NodeId, Self::Error>;

    /// Update a function if it has changed.
    fn upsert_function(&mut self, function: &UmlFunction) -> Result<Self::NodeId, Self::Error>;

    /// Update a module if it has changed.
    fn upsert_module(&mut self, module: &UmlModule) -> Result<Self::NodeId, Self::Error>;

    /// Remove a class and all its relationships from the graph.
    fn remove_class(&mut self, class_id: &ClassId) -> Result<(), Self::Error>;

    /// Remove a function and all its call edges from the graph.
    fn remove_function(&mut self, function_id: &FunctionId) -> Result<(), Self::Error>;

    /// Remove a module and all its dependency edges from the graph.
    fn remove_module(&mut self, module_name: &str) -> Result<(), Self::Error>;

    /// Synchronize the graph with a diagram.
    ///
    /// This method compares the current graph state with the diagram and
    /// applies minimal updates (insertions, updates, deletions) to synchronize them.
    fn sync_with_diagram(&mut self, diagram: &UmlDiagram) -> Result<SyncStats, Self::Error>;
}

/// Statistics from a graph synchronization operation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncStats {
    /// Number of nodes inserted.
    pub nodes_inserted: usize,
    /// Number of nodes updated.
    pub nodes_updated: usize,
    /// Number of nodes deleted.
    pub nodes_deleted: usize,
    /// Number of edges inserted.
    pub edges_inserted: usize,
    /// Number of edges deleted.
    pub edges_deleted: usize,
}
