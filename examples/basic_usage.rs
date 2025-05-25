//! Basic usage example of the task-graph library

use task_graph::{Task, TaskGraph, Context, Condition, ExtendedContext, GraphError};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct IncrementTask(i32);

#[derive(Debug, Clone)]
struct MultiplyTask(i32);

#[derive(Debug, Clone)]
struct PrintTask;

#[derive(Debug, Clone)]
struct StoreResultTask;

#[derive(Debug, Clone)]
struct CheckStoreTask;

#[async_trait::async_trait]
impl Task for IncrementTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let mut ctx = context.write().await;
        // Get current counter value
        let current = ctx.get::<i32>("counter").copied().unwrap_or(0);
        let new_value = current + self.0;
        ctx.set("counter", new_value);
        println!("IncrementTask({}): {} -> {}", self.0, current, new_value);
        
        // Store the increment value in the key-value store
        ctx.set("last_increment", self.0);
        ctx.set(format!("increment_{}", self.0), "completed");
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for MultiplyTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let mut ctx = context.write().await;
        // Get current counter value
        let current = ctx.get::<i32>("counter").copied().unwrap_or(0);
        let new_value = current * self.0;
        ctx.set("counter", new_value);
        println!("MultiplyTask({}): {} -> {}", self.0, current, new_value);
        
        // Store the multiply factor and result
        ctx.set("last_multiply_factor", self.0);
        ctx.set("multiply_result", new_value);
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for PrintTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let ctx = context.read().await;
        let value = ctx.get::<i32>("counter").copied().unwrap_or(0);
        println!("PrintTask: Final value is {}", value);
        
        // Print some stored values
        println!("Stored values:");
        if let Some(last_inc) = ctx.get::<i32>("last_increment") {
            println!("  last_increment = {}", last_inc);
        }
        if let Some(multiply_result) = ctx.get::<i32>("multiply_result") {
            println!("  multiply_result = {}", multiply_result);
        }
        if let Some(category) = ctx.get::<String>("result_category") {
            println!("  result_category = {}", category);
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for StoreResultTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let mut ctx = context.write().await;
        let value = ctx.get::<i32>("counter").copied().unwrap_or(0);
        ctx.set("final_result", value);
        ctx.set("result_category", if value > 100 { "high".to_string() } else { "low".to_string() });
        println!("StoreResultTask: Stored final result {} as {}", value,
                 if value > 100 { "high" } else { "low" });
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for CheckStoreTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let ctx = context.read().await;
        
        println!("CheckStoreTask: Checking stored values...");
        
        if let Some(last_inc) = ctx.get::<i32>("last_increment") {
            println!("  Last increment was: {}", last_inc);
        }
        
        if let Some(multiply_result) = ctx.get::<i32>("multiply_result") {
            println!("  Multiply result: {}", multiply_result);
        }
        
        if let Some(category) = ctx.get::<String>("result_category") {
            println!("  Result category: {}", category);
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Task Graph Example ===");
    
    let mut graph = TaskGraph::new();
    // Initialize counter
    {
        let context = graph.context();
        let mut ctx = context.write().await;
        ctx.set("counter", 0i32);
    }

    // Create a simple chain: Increment(5) -> Multiply(2) -> Print
    let multiply_task = MultiplyTask(2);
    graph
        .add_edge(IncrementTask(5), multiply_task.clone())?
        .add_edge(multiply_task, PrintTask)?;

    println!("Executing task graph...");
    graph.execute().await?;

    // Check final result
    let ctx = graph.context();
    let ctx_guard = ctx.read().await;
    let final_value = ctx_guard.get::<i32>("counter").copied().unwrap_or(0);
    println!("Expected: 10, Got: {}", final_value);
    assert_eq!(final_value, 10); // (0 + 5) * 2 = 10
    drop(ctx_guard);

    println!("\n=== Parallel Execution Example ===");
    
    let mut graph2 = TaskGraph::new();
    // Initialize counter
    {
        let context = graph2.context();
        let mut ctx = context.write().await;
        ctx.set("counter", 0i32);
    }

    // Create parallel branches:
    // IncrementTask(1) -> IncrementTask(10)
    //                  -> IncrementTask(100)
    // Both branches then -> PrintTask
    let increment1 = IncrementTask(1);
    let increment10 = IncrementTask(10);
    let increment100 = IncrementTask(100);
    let print_task2 = PrintTask;
    
    graph2
        .add_edge(increment1.clone(), increment10.clone())?
        .add_edge(increment1, increment100.clone())?
        .add_edge(increment10, print_task2.clone())?
        .add_edge(increment100, print_task2)?;

    println!("Executing parallel task graph...");
    graph2.execute().await?;

    // Check final result
    let ctx2 = graph2.context();
    let ctx_guard2 = ctx2.read().await;
    let final_value2 = ctx_guard2.get::<i32>("counter").copied().unwrap_or(0);
    println!("Expected: 111, Got: {}", final_value2);
    assert_eq!(final_value2, 111); // 1 + 10 + 100 = 111
    drop(ctx_guard2);

    println!("\n=== Conditional Edge with Store Example ===");
    
    let mut graph3 = TaskGraph::new();
    // Initialize counter
    {
        let context = graph3.context();
        let mut ctx = context.write().await;
        ctx.set("counter", 0i32);
    }

    // Conditional logic based on stored values
    let condition: Condition = Arc::new(|context: &ExtendedContext| {
        // Check both the counter value and stored data
        let counter_value = context.get::<i32>("counter").copied().unwrap_or(0);
        // Also check if a specific key exists in the store
        let has_high_increment = context.get::<i32>("last_increment")
            .map(|v| *v >= 10)
            .unwrap_or(false);
        
        counter_value > 5 || has_high_increment
    });

    let multiply2 = MultiplyTask(2);
    let multiply10 = MultiplyTask(10);
    
    graph3
        .add_edge(IncrementTask(10), StoreResultTask)?
        .add_edge(StoreResultTask, multiply2.clone())?
        .add_cond_edge(multiply2, CheckStoreTask, condition, Some(multiply10))?;

    println!("Executing conditional task graph with store...");
    graph3.execute().await?;

    println!("\n=== Advanced Store Usage Example ===");
    
    let mut graph4 = TaskGraph::new();
    // Initialize counter
    {
        let context = graph4.context();
        let mut ctx = context.write().await;
        ctx.set("counter", 0i32);
    }

    // Create a condition that checks multiple stored values
    let advanced_condition: Condition = Arc::new(|context: &ExtendedContext| {
        // Check if both increment tasks have completed
        let inc5_done = context.contains_key("increment_5");
        let inc10_done = context.contains_key("increment_10");
        
        // Check if the multiply result exists and is greater than 50
        let multiply_high = context.get::<i32>("multiply_result")
            .map(|v| *v > 50)
            .unwrap_or(false);
        
        inc5_done && inc10_done && multiply_high
    });

    // Build a more complex graph
    let inc5 = IncrementTask(5);
    let inc10 = IncrementTask(10);
    let mult5 = MultiplyTask(5);
    let store = StoreResultTask;
    let check = CheckStoreTask;
    let print = PrintTask;
    
    graph4
        .add_edge(inc5.clone(), inc10.clone())?
        .add_edge(inc10, mult5.clone())?
        .add_edge(mult5, store.clone())?
        .add_cond_edge(store, check.clone(), advanced_condition, Some(print.clone()))?
        .add_edge(check, print)?;

    println!("Executing advanced store usage graph...");
    graph4.execute().await?;

    println!("\nTask graph examples completed successfully!");
    Ok(())
}
