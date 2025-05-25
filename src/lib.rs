//! # Task Graph
//!
//! A library for executing tasks in a Directed Acyclic Graph (DAG) with parallel execution support.
//!
//! ## Features
//!
//! - Define tasks as traits with a single `run` method
//! - Chain tasks with direct or conditional edges
//! - Parallel execution when possible
//! - Shared mutable context between tasks
//! - Type-safe task definitions
//! - Key-value storage in context for sharing data between tasks
//!
//! ## Example
//!
//! ```rust
//! use task_graph::{Task, TaskGraph, Context, ExtendedContext, ContextExt, GraphError};
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! #[derive(Debug)]
//! struct TaskA;
//!
//! #[derive(Debug)]
//! struct TaskB;
//!
//! #[async_trait::async_trait]
//! impl Task for TaskA {
//!     async fn run(&self, context: Context) -> Result<(), GraphError> {
//!         // Simple API - no need to manually handle locks
//!         context.set("task_a_result", "completed").await;
//!         context.set("counter", 42i32).await;
//!         Ok(())
//!     }
//! }
//!
//! #[async_trait::async_trait]
//! impl Task for TaskB {
//!     async fn run(&self, context: Context) -> Result<(), GraphError> {
//!         // Simple API for reading values
//!         if let Some(result) = context.get::<String>("task_a_result").await {
//!             println!("Task A result: {}", result);
//!         }
//!         
//!         // Or use get_or_default for convenience
//!         let counter = context.get_or_default::<i32>("counter").await;
//!         println!("Counter value: {}", counter);
//!         
//!         Ok(())
//!     }
//! }
//! ```

use futures::future::join_all;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors that can occur during graph execution
#[derive(Error, Debug)]
pub enum GraphError {
    #[error("Cycle detected in graph")]
    CycleDetected,
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Invalid condition")]
    InvalidCondition,
}

/// Extended context that provides a key-value store for sharing data between tasks
pub struct ExtendedContext {
    /// Key-value store for sharing any type of data between tasks
    store: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ExtendedContext {
    /// Create a new empty extended context
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    /// Create a new extended context with initial data (for backwards compatibility)
    pub fn with_data<T: Any + Send + Sync + 'static>(data: T) -> Self {
        let mut ctx = Self::new();
        ctx.set("__legacy_data", data);
        ctx
    }

    /// Set a value in the key-value store
    /// The value can be of any type that implements Any + Send + Sync
    pub fn set<T: Any + Send + Sync + 'static>(&mut self, key: impl Into<String>, value: T) {
        self.store.insert(key.into(), Box::new(value));
    }

    /// Get a value from the key-value store with type checking
    /// Returns None if the key doesn't exist or if the type doesn't match
    pub fn get<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        self.store.get(key)?.downcast_ref::<T>()
    }

    /// Get a mutable reference to a value from the key-value store with type checking
    /// Returns None if the key doesn't exist or if the type doesn't match
    pub fn get_mut<T: Any + Send + Sync + 'static>(&mut self, key: &str) -> Option<&mut T> {
        self.store.get_mut(key)?.downcast_mut::<T>()
    }

    /// Remove a value from the key-value store
    /// Returns the value if it exists and matches the expected type
    pub fn remove<T: Any + Send + Sync + 'static>(&mut self, key: &str) -> Option<T> {
        let value = self.store.remove(key)?;
        match value.downcast::<T>() {
            Ok(boxed) => Some(*boxed),
            Err(_) => None,
        }
    }

    /// Check if a key exists in the store
    pub fn contains_key(&self, key: &str) -> bool {
        self.store.contains_key(key)
    }

    /// Get all keys in the store
    pub fn keys(&self) -> Vec<&String> {
        self.store.keys().collect()
    }

    /// Clear all values from the store
    pub fn clear(&mut self) {
        self.store.clear();
    }

    /// Get a reference to the legacy user data with downcasting (for backwards compatibility)
    #[deprecated(note = "Use get() with a key instead")]
    pub fn get_data<T: Any + Send + Sync + 'static>(&self) -> Option<&T> {
        self.get("__legacy_data")
    }

    /// Get a mutable reference to the legacy user data with downcasting (for backwards compatibility)
    #[deprecated(note = "Use get_mut() with a key instead")]
    pub fn get_data_mut<T: Any + Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.get_mut("__legacy_data")
    }
}

impl Default for ExtendedContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ExtendedContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtendedContext")
            .field("keys", &self.keys())
            .finish()
    }
}

/// A shared context that can be passed between tasks
pub type Context = Arc<RwLock<ExtendedContext>>;

/// Extension trait for Context that provides simplified access methods
#[async_trait::async_trait]
pub trait ContextExt {
    /// Set a value in the context
    async fn set<T: Any + Send + Sync + 'static>(&self, key: impl Into<String> + Send, value: T);

    /// Get a value from the context
    async fn get<T: Any + Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T>;

    /// Get a value from the context or a default if not found
    async fn get_or_default<T: Any + Send + Sync + Clone + Default + 'static>(
        &self,
        key: &str,
    ) -> T;

    /// Get a value from the context or a specific default value if not found
    async fn get_or<T: Any + Send + Sync + Clone + 'static>(&self, key: &str, default: T) -> T;

    /// Remove a value from the context
    async fn remove<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<T>;

    /// Check if a key exists in the context
    async fn contains_key(&self, key: &str) -> bool;

    /// Get all keys in the context
    async fn keys(&self) -> Vec<String>;

    /// Clear all values from the context
    async fn clear(&self);

    /// Update a value in the context using a closure
    async fn update<T, F>(&self, key: &str, updater: F) -> Result<(), GraphError>
    where
        T: Any + Send + Sync + Clone + 'static,
        F: FnOnce(T) -> T + Send;

    /// Update a value in the context or insert a default if it doesn't exist
    async fn update_or_insert<T, F>(&self, key: impl Into<String> + Send, default: T, updater: F)
    where
        T: Any + Send + Sync + Clone + 'static,
        F: FnOnce(T) -> T + Send;

    /// Execute a closure with read access to the context
    async fn with_read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ExtendedContext) -> R + Send,
        R: Send;

    /// Execute a closure with write access to the context
    async fn with_write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ExtendedContext) -> R + Send,
        R: Send;
}

#[async_trait::async_trait]
impl ContextExt for Context {
    async fn set<T: Any + Send + Sync + 'static>(&self, key: impl Into<String> + Send, value: T) {
        let mut ctx = self.write().await;
        ctx.set(key, value);
    }

    async fn get<T: Any + Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T> {
        let ctx = self.read().await;
        ctx.get::<T>(key).cloned()
    }

    async fn get_or_default<T: Any + Send + Sync + Clone + Default + 'static>(
        &self,
        key: &str,
    ) -> T {
        self.get(key).await.unwrap_or_default()
    }

    async fn get_or<T: Any + Send + Sync + Clone + 'static>(&self, key: &str, default: T) -> T {
        self.get(key).await.unwrap_or(default)
    }

    async fn remove<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<T> {
        let mut ctx = self.write().await;
        ctx.remove::<T>(key)
    }

    async fn contains_key(&self, key: &str) -> bool {
        let ctx = self.read().await;
        ctx.contains_key(key)
    }

    async fn keys(&self) -> Vec<String> {
        let ctx = self.read().await;
        ctx.keys().into_iter().map(|k| k.clone()).collect()
    }

    async fn clear(&self) {
        let mut ctx = self.write().await;
        ctx.clear();
    }

    async fn update<T, F>(&self, key: &str, updater: F) -> Result<(), GraphError>
    where
        T: Any + Send + Sync + Clone + 'static,
        F: FnOnce(T) -> T + Send,
    {
        let mut ctx = self.write().await;
        if let Some(value) = ctx.get::<T>(key).cloned() {
            let new_value = updater(value);
            ctx.set(key, new_value);
            Ok(())
        } else {
            Err(GraphError::TaskExecutionFailed(format!(
                "Key '{}' not found",
                key
            )))
        }
    }

    async fn update_or_insert<T, F>(&self, key: impl Into<String> + Send, default: T, updater: F)
    where
        T: Any + Send + Sync + Clone + 'static,
        F: FnOnce(T) -> T + Send,
    {
        let key = key.into();
        let mut ctx = self.write().await;
        let value = ctx.get::<T>(&key).cloned().unwrap_or(default);
        let new_value = updater(value);
        ctx.set(key, new_value);
    }

    async fn with_read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ExtendedContext) -> R + Send,
        R: Send,
    {
        let ctx = self.read().await;
        f(&*ctx)
    }

    async fn with_write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ExtendedContext) -> R + Send,
        R: Send,
    {
        let mut ctx = self.write().await;
        f(&mut *ctx)
    }
}

/// Trait that all tasks must implement
#[async_trait::async_trait]
pub trait Task: Send + Sync + Debug {
    /// Execute the task with the given context
    async fn run(&self, context: Context) -> Result<(), GraphError>;

    /// Get a unique identifier for this task
    fn id(&self) -> String {
        format!("{:?}", self)
    }
}

/// A condition function that determines whether an edge should be followed
/// The condition has access to the full ExtendedContext including the key-value store
pub type Condition = Arc<dyn Fn(&ExtendedContext) -> bool + Send + Sync>;

/// Represents an edge in the graph
#[derive(Clone)]
pub enum Edge {
    /// Direct edge - always followed
    Direct,
    /// Conditional edge - followed only if condition returns true
    Conditional {
        condition: Condition,
        else_task: Option<String>,
    },
}

impl Debug for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edge::Direct => write!(f, "Direct"),
            Edge::Conditional { else_task, .. } => {
                write!(
                    f,
                    "Conditional {{ condition: <function>, else_task: {:?} }}",
                    else_task
                )
            }
        }
    }
}

/// A node in the task graph
#[derive(Debug)]
struct TaskNode {
    task: Arc<dyn Task>,
    dependencies: HashSet<String>,
    dependents: Vec<(String, Edge)>,
}

/// The main task graph structure
pub struct TaskGraph {
    nodes: HashMap<String, TaskNode>,
    context: Context,
}

impl TaskGraph {
    /// Create a new task graph with an empty context
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            context: Arc::new(RwLock::new(ExtendedContext::new())),
        }
    }

    /// Create a new task graph with initial data (for backwards compatibility)
    pub fn with_data<T: Any + Send + Sync + 'static>(data: T) -> Self {
        Self {
            nodes: HashMap::new(),
            context: Arc::new(RwLock::new(ExtendedContext::with_data(data))),
        }
    }

    /// Add a task to the graph
    pub fn add_task<T: Task + 'static>(&mut self, task: T) -> &mut Self {
        let task_id = task.id();
        // Only insert if the task doesn't already exist
        if !self.nodes.contains_key(&task_id) {
            let task_node = TaskNode {
                task: Arc::new(task),
                dependencies: HashSet::new(),
                dependents: Vec::new(),
            };
            self.nodes.insert(task_id, task_node);
        }
        self
    }

    /// Add a direct edge between two tasks
    pub fn add_edge<A: Task + 'static, B: Task + 'static>(
        &mut self,
        from_task: A,
        to_task: B,
    ) -> Result<&mut Self, GraphError> {
        let from_id = from_task.id();
        let to_id = to_task.id();

        // Add tasks if they don't exist
        self.add_task(from_task);
        self.add_task(to_task);

        // Add edge
        self.add_edge_by_id(&from_id, &to_id, Edge::Direct)?;
        Ok(self)
    }

    /// Add a conditional edge between two tasks
    pub fn add_cond_edge<A: Task + 'static, B: Task + 'static, E: Task + 'static>(
        &mut self,
        from_task: A,
        to_task: B,
        condition: Condition,
        else_task: Option<E>,
    ) -> Result<&mut Self, GraphError> {
        let from_id = from_task.id();
        let to_id = to_task.id();

        // Add tasks if they don't exist
        self.add_task(from_task);
        self.add_task(to_task);

        let else_id = if let Some(else_task) = else_task {
            let else_id = else_task.id();
            self.add_task(else_task);
            Some(else_id)
        } else {
            None
        };

        // Add conditional edge
        let edge = Edge::Conditional {
            condition,
            else_task: else_id.clone(),
        };
        self.add_edge_by_id(&from_id, &to_id, edge)?;

        // Also add dependency for the else task if it exists
        if let Some(else_id) = else_id {
            if let Some(else_node) = self.nodes.get_mut(&else_id) {
                else_node.dependencies.insert(from_id);
            }
        }

        Ok(self)
    }

    /// Add an edge between tasks by their IDs
    fn add_edge_by_id(&mut self, from_id: &str, to_id: &str, edge: Edge) -> Result<(), GraphError> {
        // Check if both tasks exist
        if !self.nodes.contains_key(from_id) {
            return Err(GraphError::TaskNotFound(from_id.to_string()));
        }
        if !self.nodes.contains_key(to_id) {
            return Err(GraphError::TaskNotFound(to_id.to_string()));
        }

        // Add the edge
        if let Some(from_node) = self.nodes.get_mut(from_id) {
            from_node.dependents.push((to_id.to_string(), edge));
        }

        if let Some(to_node) = self.nodes.get_mut(to_id) {
            to_node.dependencies.insert(from_id.to_string());
        }

        // Check for cycles
        if self.has_cycle() {
            return Err(GraphError::CycleDetected);
        }

        Ok(())
    }

    /// Check if the graph has cycles using DFS
    fn has_cycle(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                if self.has_cycle_util(node_id, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }
        false
    }

    fn has_cycle_util(
        &self,
        node_id: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        if let Some(node) = self.nodes.get(node_id) {
            for (dependent_id, _) in &node.dependents {
                if !visited.contains(dependent_id) {
                    if self.has_cycle_util(dependent_id, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dependent_id) {
                    return true;
                }
            }
        }

        rec_stack.remove(node_id);
        false
    }

    /// Execute the task graph
    pub async fn execute(&self) -> Result<(), GraphError> {
        let mut completed = HashSet::new();
        let mut in_progress = HashSet::new();
        let mut blocked_by_condition = HashSet::new();

        while completed.len() < self.nodes.len() {
            // Find tasks that are ready to execute
            let ready_tasks: Vec<String> = self
                .nodes
                .iter()
                .filter(|(id, _node)| {
                    !completed.contains(*id)
                        && !in_progress.contains(*id)
                        && !blocked_by_condition.contains(*id)
                        && self.is_task_ready(id, &completed)
                })
                .map(|(id, _)| id.clone())
                .collect();

            if ready_tasks.is_empty() {
                break; // No more tasks can be executed
            }

            // Execute ready tasks in parallel
            let mut task_futures = Vec::new();
            for task_id in &ready_tasks {
                in_progress.insert(task_id.clone());
                if let Some(node) = self.nodes.get(task_id) {
                    let task = node.task.clone();
                    let context = self.context.clone();
                    let task_id_clone = task_id.clone();

                    let future = async move {
                        let result = task.run(context).await;
                        (task_id_clone, result)
                    };
                    task_futures.push(future);
                }
            }

            // Wait for all tasks to complete
            let results = join_all(task_futures).await;

            for (task_id, result) in results {
                in_progress.remove(&task_id);
                match result {
                    Ok(_) => {
                        completed.insert(task_id.clone());

                        // Handle conditional edges after task completion
                        if let Some(node) = self.nodes.get(&task_id) {
                            for (dependent_id, edge) in &node.dependents {
                                match edge {
                                    Edge::Direct => {
                                        // Direct edges are always followed
                                    }
                                    Edge::Conditional {
                                        condition,
                                        else_task,
                                    } => {
                                        // Evaluate condition with access to the full context
                                        let ctx = self.context.read().await;
                                        let should_follow = condition(&*ctx);
                                        drop(ctx); // Release the lock

                                        if !should_follow {
                                            // Block the main conditional task
                                            blocked_by_condition.insert(dependent_id.clone());
                                            // If there's an else task, make sure it can run
                                            if let Some(else_id) = else_task {
                                                blocked_by_condition.remove(else_id);
                                            }
                                        } else {
                                            // Block the else task if condition is true
                                            if let Some(else_id) = else_task {
                                                blocked_by_condition.insert(else_id.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(GraphError::TaskExecutionFailed(format!(
                            "Task {} failed: {}",
                            task_id, e
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a task is ready to execute based on its dependencies
    fn is_task_ready(&self, task_id: &str, completed: &HashSet<String>) -> bool {
        if let Some(node) = self.nodes.get(task_id) {
            node.dependencies.iter().all(|dep| completed.contains(dep))
        } else {
            false
        }
    }

    /// Get the current context
    pub fn context(&self) -> Context {
        self.context.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TaskA;

    #[derive(Debug)]
    struct TaskB;

    #[derive(Debug)]
    struct TaskC;

    #[derive(Debug)]
    struct TaskD;

    #[async_trait::async_trait]
    impl Task for TaskA {
        async fn run(&self, context: Context) -> Result<(), GraphError> {
            let mut ctx = context.write().await;
            // Store counter value
            ctx.set("counter", 1i32);
            // Store execution order
            if let Some(order) = ctx.get_mut::<Vec<String>>("execution_order") {
                order.push("A".to_string());
            } else {
                ctx.set("execution_order", vec!["A".to_string()]);
            }
            // Store some data in the key-value store
            ctx.set("task_a_completed", true);
            ctx.set("task_a_value", 1i32);
            drop(ctx);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Task for TaskB {
        async fn run(&self, context: Context) -> Result<(), GraphError> {
            let mut ctx = context.write().await;
            // Update counter
            let current = ctx.get::<i32>("counter").copied().unwrap_or(0);
            ctx.set("counter", current + 10);
            // Update execution order
            let has_order = ctx.contains_key("execution_order");
            if has_order {
                if let Some(order) = ctx.get_mut::<Vec<String>>("execution_order") {
                    order.push("B".to_string());
                }
            } else {
                ctx.set("execution_order", vec!["B".to_string()]);
            }
            // Check if task A completed
            if ctx.get::<bool>("task_a_completed").is_some() {
                ctx.set("task_b_saw_a", true);
            }
            drop(ctx);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Task for TaskC {
        async fn run(&self, context: Context) -> Result<(), GraphError> {
            let mut ctx = context.write().await;
            // Update counter
            let current = ctx.get::<i32>("counter").copied().unwrap_or(0);
            ctx.set("counter", current + 100);
            // Update execution order
            if let Some(order) = ctx.get_mut::<Vec<String>>("execution_order") {
                order.push("C".to_string());
            }
            ctx.set("task_c_completed", true);
            drop(ctx);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl Task for TaskD {
        async fn run(&self, context: Context) -> Result<(), GraphError> {
            let mut ctx = context.write().await;
            // Update counter
            let current = ctx.get::<i32>("counter").copied().unwrap_or(0);
            ctx.set("counter", current * 2);
            // Update execution order
            if let Some(order) = ctx.get_mut::<Vec<String>>("execution_order") {
                order.push("D".to_string());
            }
            ctx.set("task_d_completed", true);
            drop(ctx);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_simple_chain() {
        let mut graph = TaskGraph::new();

        graph.add_edge(TaskA, TaskB).unwrap();

        graph.execute().await.unwrap();

        let ctx = graph.context();
        let ctx_guard = ctx.read().await;
        let final_value = ctx_guard.get::<i32>("counter").copied().unwrap_or(0);
        assert_eq!(final_value, 11); // 1 + 10 = 11
        let order = ctx_guard.get::<Vec<String>>("execution_order").unwrap();
        assert_eq!(order, &vec!["A", "B"]);

        // Check key-value store
        assert_eq!(ctx_guard.get::<bool>("task_a_completed"), Some(&true));
        assert_eq!(ctx_guard.get::<bool>("task_b_saw_a"), Some(&true));
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let mut graph = TaskGraph::new();

        // A -> B and A -> C (B and C should run in parallel)
        graph.add_edge(TaskA, TaskB).unwrap();
        graph.add_edge(TaskA, TaskC).unwrap();

        graph.execute().await.unwrap();

        let ctx = graph.context();
        let ctx_guard = ctx.read().await;
        let final_value = ctx_guard.get::<i32>("counter").copied().unwrap_or(0);
        assert_eq!(final_value, 111); // 1 + 10 + 100 = 111
        let order = ctx_guard.get::<Vec<String>>("execution_order").unwrap();
        // B and C should both be in the order, but their relative order is not deterministic
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "A");
        assert!(order.contains(&"B".to_string()));
        assert!(order.contains(&"C".to_string()));
    }

    #[tokio::test]
    async fn test_conditional_edge() {
        let mut graph = TaskGraph::new();

        let condition: Condition = Arc::new(|context: &ExtendedContext| {
            context.get::<i32>("counter").copied().unwrap_or(0) > 5
        });

        graph.add_edge(TaskA, TaskB).unwrap();
        graph
            .add_cond_edge(TaskB, TaskC, condition, Some(TaskD))
            .unwrap();

        graph.execute().await.unwrap();

        // Check final result - should have executed A, B, C (not D)
        let ctx = graph.context();
        let ctx_guard = ctx.read().await;
        let final_value = ctx_guard.get::<i32>("counter").copied().unwrap_or(0);
        assert_eq!(final_value, 111); // 1 + 10 + 100 = 111
        let order = ctx_guard.get::<Vec<String>>("execution_order").unwrap();
        assert_eq!(order, &vec!["A", "B", "C"]);
    }

    #[tokio::test]
    async fn test_conditional_with_store_access() {
        let mut graph = TaskGraph::new();

        // Condition that checks the key-value store
        let condition: Condition =
            Arc::new(|context: &ExtendedContext| context.get::<bool>("task_a_completed").is_some());

        graph.add_edge(TaskA, TaskB).unwrap();
        graph
            .add_cond_edge(TaskB, TaskC, condition, Some(TaskD))
            .unwrap();

        graph.execute().await.unwrap();

        // Check that task B saw task A's completion
        let ctx = graph.context();
        let ctx_guard = ctx.read().await;
        assert_eq!(ctx_guard.get::<bool>("task_b_saw_a"), Some(&true));

        // Since task A stores "task_a_completed" = true, the condition should be true
        // So TaskC should run, not TaskD
        assert_eq!(ctx_guard.get::<bool>("task_c_completed"), Some(&true));
        assert_eq!(ctx_guard.get::<bool>("task_d_completed"), None);
    }

    #[tokio::test]
    async fn test_cycle_detection() {
        let mut graph = TaskGraph::new();

        // Try to create a cycle: A -> B -> C -> A
        graph.add_edge(TaskA, TaskB).unwrap();
        graph.add_edge(TaskB, TaskC).unwrap();
        let result = graph.add_edge(TaskC, TaskA);

        assert!(matches!(result, Err(GraphError::CycleDetected)));
    }
}
