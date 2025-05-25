# Task Graph

A Rust library for executing tasks in a Directed Acyclic Graph (DAG) with parallel execution support.

## Features

- **Task Trait**: Define tasks as structs implementing a simple `Task` trait with a single `run` method
- **Type-Safe Key-Value Store**: Store and retrieve any type implementing `Any + Send + Sync` using string keys
- **No Custom Context Required**: Simply use string keys to share data between tasks
- **Direct Edges**: Chain tasks with direct dependencies
- **Conditional Edges**: Add conditional logic to determine which tasks to execute based on runtime conditions
- **Parallel Execution**: Automatically execute independent tasks in parallel for optimal performance
- **Cycle Detection**: Prevent infinite loops by detecting cycles in the task graph
- **Type Safety**: Leverage Rust's type system for safe task definitions
- **Async Support**: Built on top of Tokio for async task execution

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
task-graph = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
async-trait = "0.1"
```

### Optional Features

For web service examples using Axum:

```toml
[dependencies]
task-graph = { version = "0.1.0", features = ["axum-example"] }
```

The `axum-example` feature includes:
- `axum` - Web framework
- `serde` - Serialization/deserialization
- `serde_json` - JSON support
- `chrono` - Date/time handling

## Quick Start

```rust
use task_graph::{Task, TaskGraph, Context, ExtendedContext};
use std::sync::Arc;

#[derive(Debug)]
struct IncrementTask(i32);

#[async_trait::async_trait]
impl Task for IncrementTask {
    async fn run(&self, context: Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ctx = context.write().await;
        
        // Get current counter value (or default to 0)
        let current = ctx.get::<i32>("counter").copied().unwrap_or(0);
        let new_value = current + self.0;
        
        // Store the new value
        ctx.set("counter", new_value);
        println!("Added {}: {} -> {}", self.0, current, new_value);
        
        // Store additional data
        ctx.set("last_increment", self.0);
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = TaskGraph::new();

    // Create a simple chain: Add 5, then Add 10
    graph.add_edge(IncrementTask(5), IncrementTask(10))?;

    graph.execute().await?;

    // Check the result
    let ctx = graph.context();
    let ctx_guard = ctx.read().await;
    
    let final_value = ctx_guard.get::<i32>("counter").copied().unwrap_or(0);
    println!("Final value: {}", final_value); // 15
    
    if let Some(last_inc) = ctx_guard.get::<i32>("last_increment") {
        println!("Last increment was: {}", last_inc); // 10
    }

    Ok(())
}
```

## Core Concepts

### Tasks

Tasks are the building blocks of your execution graph. Each task implements the `Task` trait:

```rust
#[async_trait::async_trait]
pub trait Task: Send + Sync + Debug {
    async fn run(&self, context: Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    
    fn id(&self) -> String {
        format!("{:?}", self)
    }
}
```

### Context

The context provides a type-safe key-value store for sharing data between tasks:

```rust
pub type Context = Arc<RwLock<ExtendedContext>>;
```

You can store and retrieve any type that implements `Any + Send + Sync`:

```rust
// In a task's run method:
let mut ctx = context.write().await;

// Store any type
ctx.set("my_string", "hello world");
ctx.set("my_number", 42i32);
ctx.set("my_vec", vec![1, 2, 3]);
ctx.set("my_bool", true);

// Retrieve with type safety
let my_string = ctx.get::<&str>("my_string");
let my_number = ctx.get::<i32>("my_number");
let my_vec = ctx.get::<Vec<i32>>("my_vec");
let my_bool = ctx.get::<bool>("my_bool");
```

### Task Graph

The `TaskGraph` manages the execution of tasks according to their dependencies:

```rust
let mut graph = TaskGraph::new();
```

### Adding Edges

#### Direct Edges

Direct edges create a dependency where the second task runs after the first:

```rust
graph.add_edge(TaskA, TaskB)?; // TaskB runs after TaskA
```

#### Conditional Edges

Conditional edges allow you to add branching logic:

```rust
let condition = Arc::new(|context: &ExtendedContext| {
    // Check stored values
    context.get::<i32>("counter")
        .map(|v| *v > 10)
        .unwrap_or(false)
});

graph.add_cond_edge(TaskA, TaskB, condition, Some(TaskC))?;
// If condition is true, TaskB runs after TaskA
// If condition is false, TaskC runs after TaskA
```

## Examples

### Parallel Execution

```rust
let mut graph = TaskGraph::new();

// TaskA runs first, then TaskB and TaskC run in parallel
graph.add_edge(TaskA, TaskB)?;
graph.add_edge(TaskA, TaskC)?;

graph.execute().await?;
```

### Complex Workflow

```rust
let mut graph = TaskGraph::new();

// Create a more complex workflow
graph.add_edge(DataLoader, DataValidator)?;
graph.add_edge(DataValidator, DataProcessor)?;
graph.add_edge(DataValidator, DataBackup)?; // Runs in parallel with DataProcessor

// Conditional logic based on processing results
let condition = Arc::new(|context: &ExtendedContext| {
    // Check if processing was successful
    context.get::<String>("processing_status")
        .map(|s| s == "success")
        .unwrap_or(false)
});

graph.add_cond_edge(DataProcessor, SuccessHandler, condition, Some(ErrorHandler))?;

graph.execute().await?;
```

### Using the Type-Safe Store

```rust
#[derive(Debug)]
struct ProcessingTask;

#[async_trait::async_trait]
impl Task for ProcessingTask {
    async fn run(&self, context: Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut ctx = context.write().await;
        
        // Store various types
        ctx.set("items_processed", 150i32);
        ctx.set("processing_time_ms", 2500u64);
        ctx.set("status", "completed".to_string());
        ctx.set("errors", Vec::<String>::new());
        ctx.set("success_rate", 0.95f64);
        
        Ok(())
    }
}

#[derive(Debug)]
struct ReportingTask;

#[async_trait::async_trait]
impl Task for ReportingTask {
    async fn run(&self, context: Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ctx = context.read().await;
        
        // Read data with type safety
        if let Some(count) = ctx.get::<i32>("items_processed") {
            println!("Processed {} items", count);
        }
        
        if let Some(time) = ctx.get::<u64>("processing_time_ms") {
            println!("Processing took {} ms", time);
        }
        
        if let Some(rate) = ctx.get::<f64>("success_rate") {
            println!("Success rate: {:.2}%", rate * 100.0);
        }
        
        Ok(())
    }
}
```

Available context methods:
- `set<T>(key, value)` - Store a value of any type
- `get<T>(key)` - Retrieve a value with type checking
- `get_mut<T>(key)` - Get a mutable reference to a value
- `remove<T>(key)` - Remove and return a value
- `contains_key(key)` - Check if a key exists
- `keys()` - Get all keys in the store
- `clear()` - Remove all key-value pairs

## Error Handling

The library provides comprehensive error handling:

```rust
pub enum GraphError {
    CycleDetected,
    TaskExecutionFailed(String),
    TaskNotFound(String),
    InvalidCondition,
}
```

Tasks can return errors, and the graph execution will stop and propagate the error:

```rust
#[async_trait::async_trait]
impl Task for MyTask {
    async fn run(&self, context: Context) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Task logic here
        if some_error_condition {
            return Err("Something went wrong".into());
        }
        Ok(())
    }
}
```

## Running Examples

Run the basic example:

```bash
cargo run --example basic_usage
```

Run the advanced conditional execution example:

```bash
cargo run --example advanced_conditional
```

Run the Axum web service example (requires the `axum-example` feature):

```bash
cargo run --example axum_service --features axum-example
```

### Advanced Example Features

The `advanced_conditional` example demonstrates:

- **Complex Graph Structure**: A -> (B, C) -> D | ErrorHandler
- **Parallel Execution**: Tasks B and C run simultaneously after A completes
- **Conditional Logic**: Task D runs only if specific conditions are met, otherwise ErrorHandler runs
- **Real-world Simulation**: Simulates a data processing pipeline with CPU and I/O operations
- **Context-based Decisions**: Conditions evaluate multiple stored values from the context
- **Error Handling**: Graceful fallback to alternative execution paths
- **Performance Monitoring**: Tracks execution times and efficiency metrics

The example shows two scenarios:
1. **Success Path**: Large dataset triggers high-performance processing leading to successful aggregation
2. **Error Path**: Small dataset results in lower scores, triggering the error handling path

This demonstrates how the library can handle complex real-world workflows with branching logic and parallel processing.

## Running Tests

```bash
cargo test
```

## Context Management Best Practices

When working with the shared context, follow these patterns for optimal performance and correctness:

### ✅ Quick Read Operations
For reading single values, use scoped access to minimize lock duration:
```rust
let data_size = {
    let ctx = context.read().await;
    ctx.get::<usize>("data_size").copied().unwrap_or(0)
};
```

### ✅ Grouped Write Operations
When setting multiple related values, group them in a single scope:
```rust
{
    let mut ctx = context.write().await;
    ctx.set("processed_items", items);
    ctx.set("processing_time_ms", time_ms);
    ctx.set("task_completed", true);
}
```

### ✅ Explicit Lock Release
For long-running computations, explicitly release locks before heavy work:
```rust
let ctx = context.read().await;
let values = /* read what you need */;
drop(ctx); // Release lock before computation

// Heavy computation here
sleep(Duration::from_millis(300)).await;

// Acquire lock again for writes if needed
{
    let mut ctx = context.write().await;
    ctx.set("result", computed_value);
}
```

### ❌ Anti-Patterns to Avoid
```rust
// Bad: Holding lock during async operations
let mut ctx = context.write().await;
ctx.set("start", true);
sleep(Duration::from_millis(1000)).await; // Lock held during sleep!
ctx.set("end", true);

// Bad: Multiple separate acquisitions for related data
let mut ctx1 = context.write().await;
ctx1.set("value1", x);
drop(ctx1);
let mut ctx2 = context.write().await; // Unnecessary separate lock
ctx2.set("value2", y);
```

**Key Principle**: Hold locks for the shortest time necessary while ensuring data consistency.

## Safety and Concurrency

- **No Unsafe Code**: The library is built without using any unsafe Rust code
- **Thread Safe**: All operations are thread-safe using Rust's ownership system and async primitives
- **Deadlock Prevention**: Careful lock ordering prevents deadlocks
- **Cycle Detection**: Prevents infinite loops by detecting cycles in the task graph

## Performance

- **Parallel Execution**: Independent tasks run concurrently for optimal performance
- **Async Runtime**: Built on Tokio for efficient async task scheduling
- **Minimal Overhead**: Lightweight abstractions with minimal runtime overhead

## Migration from Previous Versions

If you were using a custom context type, you can migrate to the new API:

```rust
// Old API
let context = MyContext { value: 0 };
let mut graph = TaskGraph::new(context);

// Access in task
if let Some(my_ctx) = ctx.get_data::<MyContext>() {
    // use my_ctx
}

// New API
let mut graph = TaskGraph::new();

// Initialize values
{
    let context = graph.context();
    let mut ctx = context.write().await;
    ctx.set("value", 0i32);
}

// Access in task
if let Some(value) = ctx.get::<i32>("value") {
    // use value
}
```

## Limitations

- **Type Erasure**: Values are stored as `Any` types, requiring runtime type checking
- **Dynamic Graph Modification**: The graph structure cannot be modified during execution
- **Conditional Edge Implementation**: The current conditional edge implementation is simplified

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
