//! Advanced conditional execution example demonstrating:
//! - Graph structure: A -> B, C and to D after condition
//! - Conditional execution based on context
//! - Parallel execution of B and C
//! - Context sharing between tasks
//! - Real-world scenario simulation

use task_graph::{Task, TaskGraph, Context, Condition, ExtendedContext, GraphError};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Task A: Data Processing Setup
/// This task initializes the processing pipeline and sets up initial data
#[derive(Debug, Clone)]
struct DataProcessingTask {
    data_size: usize,
}

/// Task B: CPU-intensive processing
/// Simulates heavy computation that can run in parallel with Task C
#[derive(Debug, Clone)]
struct CpuProcessingTask;

/// Task C: I/O-intensive processing  
/// Simulates network/disk operations that can run in parallel with Task B
#[derive(Debug, Clone)]
struct IoProcessingTask;

/// Task D: Final aggregation
/// Only runs if both B and C complete successfully and meet certain conditions
#[derive(Debug, Clone)]
struct AggregationTask;

/// Alternative Task: Error handling
/// Runs instead of Task D if conditions are not met
#[derive(Debug, Clone)]
struct ErrorHandlingTask;

/// Monitoring task to show execution progress
#[derive(Debug, Clone)]
struct MonitoringTask {
    task_name: String,
}

#[async_trait::async_trait]
impl Task for DataProcessingTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let start_time = Instant::now();
        println!("🚀 Task A (DataProcessing): Starting with {} items", self.data_size);
        
        // Simulate some initial processing time
        sleep(Duration::from_millis(500)).await;
        
        let mut ctx = context.write().await;
        
        // Store initial processing data
        ctx.set("data_size", self.data_size);
        ctx.set("processing_start_time", start_time);
        ctx.set("task_a_completed", true);
        ctx.set("processed_items", 0usize);
        ctx.set("error_count", 0usize);
        
        // Simulate data validation
        let is_valid_data = self.data_size > 0 && self.data_size <= 10000;
        ctx.set("data_valid", is_valid_data);
        
        println!("✅ Task A (DataProcessing): Completed setup for {} items (valid: {})", 
                 self.data_size, is_valid_data);
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for CpuProcessingTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("🔥 Task B (CpuProcessing): Starting CPU-intensive work");
        
        // Simulate CPU-intensive work
        let start = Instant::now();
        
        // Read data size from context
        let data_size = {
            let ctx = context.read().await;
            ctx.get::<usize>("data_size").copied().unwrap_or(0)
        };
        
        // Simulate processing time proportional to data size
        let processing_time = Duration::from_millis(100 + (data_size / 10) as u64);
        sleep(processing_time).await;
        
        // Simulate some computation results
        let processed_items = data_size / 2;
        let cpu_score = if data_size > 5000 { 95 } else { 85 };
        
        {
            let mut ctx = context.write().await;
            ctx.set("cpu_processed_items", processed_items);
            ctx.set("cpu_processing_time_ms", start.elapsed().as_millis() as u64);
            ctx.set("cpu_score", cpu_score);
            ctx.set("task_b_completed", true);
        }
        
        println!("✅ Task B (CpuProcessing): Processed {} items with score {} in {:?}", 
                 processed_items, cpu_score, start.elapsed());
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for IoProcessingTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("💾 Task C (IoProcessing): Starting I/O operations");
        
        let start = Instant::now();
        
        // Read data size from context
        let data_size = {
            let ctx = context.read().await;
            ctx.get::<usize>("data_size").copied().unwrap_or(0)
        };
        
        // Simulate I/O operations (network calls, database queries, file operations)
        for i in 0..3 {
            println!("💾 Task C (IoProcessing): I/O operation {} of 3", i + 1);
            sleep(Duration::from_millis(200)).await;
        }
        
        // Simulate I/O results
        let io_operations = 3;
        let success_rate = if data_size < 1000 { 100 } else { 95 };
        let errors = if success_rate < 100 { 1 } else { 0 };
        
        {
            let mut ctx = context.write().await;
            ctx.set("io_operations", io_operations);
            ctx.set("io_success_rate", success_rate);
            ctx.set("io_errors", errors);
            ctx.set("io_processing_time_ms", start.elapsed().as_millis() as u64);
            ctx.set("task_c_completed", true);
        }
        
        println!("✅ Task C (IoProcessing): Completed {} operations with {}% success rate in {:?}", 
                 io_operations, success_rate, start.elapsed());
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for AggregationTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("📊 Task D (Aggregation): Starting final aggregation");
        
        let ctx = context.read().await;
        
        // Gather results from both B and C
        let cpu_score = ctx.get::<i32>("cpu_score").copied().unwrap_or(0);
        let cpu_items = ctx.get::<usize>("cpu_processed_items").copied().unwrap_or(0);
        let cpu_time = ctx.get::<u64>("cpu_processing_time_ms").copied().unwrap_or(0);
        
        let io_success_rate = ctx.get::<i32>("io_success_rate").copied().unwrap_or(0);
        let io_operations = ctx.get::<i32>("io_operations").copied().unwrap_or(0);
        let io_time = ctx.get::<u64>("io_processing_time_ms").copied().unwrap_or(0);
        
        let data_size = ctx.get::<usize>("data_size").copied().unwrap_or(0);
        
        drop(ctx);
        
        // Simulate aggregation work
        sleep(Duration::from_millis(300)).await;
        
        // Calculate final metrics
        let total_time = cpu_time + io_time;
        let overall_score = (cpu_score + io_success_rate) / 2;
        let efficiency = if total_time > 0 { data_size as f64 / total_time as f64 * 1000.0 } else { 0.0 };
        
        {
            let mut ctx = context.write().await;
            ctx.set("final_score", overall_score);
            ctx.set("efficiency", efficiency);
            ctx.set("total_processing_time_ms", total_time);
            ctx.set("aggregation_completed", true);
        }
        
        println!("🎯 Task D (Aggregation): SUCCESS!");
        println!("   📈 Final Score: {}", overall_score);
        println!("   ⚡ Efficiency: {:.2} items/sec", efficiency);
        println!("   ⏱️  Total Time: {}ms", total_time);
        println!("   📦 Items Processed: {}", cpu_items);
        println!("   🔗 I/O Operations: {}", io_operations);
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for ErrorHandlingTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("⚠️  Alternative Task (ErrorHandling): Handling processing issues");
        
        let ctx = context.read().await;
        
        // Analyze what went wrong
        let data_valid = ctx.get::<bool>("data_valid").copied().unwrap_or(false);
        let task_b_completed = ctx.get::<bool>("task_b_completed").copied().unwrap_or(false);
        let task_c_completed = ctx.get::<bool>("task_c_completed").copied().unwrap_or(false);
        let cpu_score = ctx.get::<i32>("cpu_score").copied().unwrap_or(0);
        let io_success_rate = ctx.get::<i32>("io_success_rate").copied().unwrap_or(0);
        
        drop(ctx);
        
        println!("🔍 Error Analysis:");
        println!("   Data Valid: {}", data_valid);
        println!("   Task B Completed: {}", task_b_completed);
        println!("   Task C Completed: {}", task_c_completed);
        println!("   CPU Score: {}", cpu_score);
        println!("   I/O Success Rate: {}%", io_success_rate);
        
        // Simulate error handling work
        sleep(Duration::from_millis(200)).await;
        
        {
            let mut ctx = context.write().await;
            ctx.set("error_handled", true);
            ctx.set("recovery_attempted", true);
        }
        
        println!("🔧 Error handling completed - system in safe state");
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for MonitoringTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let ctx = context.read().await;
        let start_time = ctx.get::<Instant>("processing_start_time");
        
        if let Some(start) = start_time {
            let elapsed = start.elapsed();
            println!("📊 Monitor ({}): Elapsed time: {:?}", self.task_name, elapsed);
        } else {
            println!("📊 Monitor ({}): Monitoring active", self.task_name);
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Advanced Conditional Task Graph Example");
    println!("==========================================");
    println!("Graph Structure: A -> (B, C) -> D (if conditions met) | ErrorHandler (if not)");
    println!();
    
    // Scenario 1: Successful processing (conditions met)
    println!("📋 Scenario 1: Normal Processing (Large Dataset)");
    println!("------------------------------------------------");
    
    let mut graph1 = TaskGraph::new();
    
    // Define the condition for Task D execution
    // Task D runs only if:
    // 1. Both Task B and C completed successfully
    // 2. CPU score is good (>= 90)
    // 3. I/O success rate is high (>= 95%)
    // 4. Data was valid
    let success_condition: Condition = Arc::new(|context: &ExtendedContext| {
        let task_b_done = context.get::<bool>("task_b_completed").copied().unwrap_or(false);
        let task_c_done = context.get::<bool>("task_c_completed").copied().unwrap_or(false);
        let cpu_score = context.get::<i32>("cpu_score").copied().unwrap_or(0);
        let io_success_rate = context.get::<i32>("io_success_rate").copied().unwrap_or(0);
        let data_valid = context.get::<bool>("data_valid").copied().unwrap_or(false);
        
        let conditions_met = task_b_done && task_c_done && cpu_score >= 90 && io_success_rate >= 95 && data_valid;
        
        println!("🔍 Condition Check:");
        println!("   Task B completed: {}", task_b_done);
        println!("   Task C completed: {}", task_c_done);
        println!("   CPU score >= 90: {} ({})", cpu_score >= 90, cpu_score);
        println!("   I/O success >= 95%: {} ({}%)", io_success_rate >= 95, io_success_rate);
        println!("   Data valid: {}", data_valid);
        println!("   → Condition result: {}", conditions_met);
        
        conditions_met
    });
    
    // Build the graph: A -> B, C -> D (conditional) | ErrorHandler
    let task_a = DataProcessingTask { data_size: 8000 };
    let task_b = CpuProcessingTask;
    let task_c = IoProcessingTask;
    let task_d = AggregationTask;
    let error_handler = ErrorHandlingTask;
    let monitor1 = MonitoringTask { task_name: "After-B".to_string() };
    let monitor2 = MonitoringTask { task_name: "After-C".to_string() };
    
    graph1
        // A -> B and A -> C (parallel execution)
        .add_edge(task_a.clone(), task_b.clone())?
        .add_edge(task_a, task_c.clone())?
        // Add monitoring after B and C
        .add_edge(task_b, monitor1)?
        .add_edge(task_c, monitor2.clone())?
        // Conditional edge: monitor2 -> D (if condition) | ErrorHandler (if not)
        .add_cond_edge(monitor2, task_d, success_condition, Some(error_handler))?;
    
    println!("🚀 Executing graph...");
    let start = Instant::now();
    graph1.execute().await?;
    println!("⏱️  Total execution time: {:?}", start.elapsed());
    
    // Check final results
    {
        let ctx = graph1.context();
        let ctx_guard = ctx.read().await;
        
        if ctx_guard.get::<bool>("aggregation_completed").copied().unwrap_or(false) {
            println!("🎉 Scenario 1: SUCCESS - Aggregation completed!");
        } else if ctx_guard.get::<bool>("error_handled").copied().unwrap_or(false) {
            println!("⚠️  Scenario 1: Error path taken - but handled gracefully");
        }
    }
    
    println!("\n{}", "=".repeat(60));
    
    // Scenario 2: Error conditions (conditions not met)
    println!("📋 Scenario 2: Error Conditions (Small Dataset)");
    println!("-----------------------------------------------");
    
    let mut graph2 = TaskGraph::new();
    
    // Same condition as before
    let success_condition2: Condition = Arc::new(|context: &ExtendedContext| {
        let task_b_done = context.get::<bool>("task_b_completed").copied().unwrap_or(false);
        let task_c_done = context.get::<bool>("task_c_completed").copied().unwrap_or(false);
        let cpu_score = context.get::<i32>("cpu_score").copied().unwrap_or(0);
        let io_success_rate = context.get::<i32>("io_success_rate").copied().unwrap_or(0);
        let data_valid = context.get::<bool>("data_valid").copied().unwrap_or(false);
        
        let conditions_met = task_b_done && task_c_done && cpu_score >= 90 && io_success_rate >= 95 && data_valid;
        
        println!("🔍 Condition Check:");
        println!("   Task B completed: {}", task_b_done);
        println!("   Task C completed: {}", task_c_done);
        println!("   CPU score >= 90: {} ({})", cpu_score >= 90, cpu_score);
        println!("   I/O success >= 95%: {} ({}%)", io_success_rate >= 95, io_success_rate);
        println!("   Data valid: {}", data_valid);
        println!("   → Condition result: {}", conditions_met);
        
        conditions_met
    });
    
    // Build the same graph structure but with smaller dataset (will trigger error path)
    let task_a2 = DataProcessingTask { data_size: 500 }; // Small dataset -> lower scores
    let task_b2 = CpuProcessingTask;
    let task_c2 = IoProcessingTask;
    let task_d2 = AggregationTask;
    let error_handler2 = ErrorHandlingTask;
    let monitor3 = MonitoringTask { task_name: "After-B".to_string() };
    let monitor4 = MonitoringTask { task_name: "After-C".to_string() };
    
    graph2
        .add_edge(task_a2.clone(), task_b2.clone())?
        .add_edge(task_a2, task_c2.clone())?
        .add_edge(task_b2, monitor3)?
        .add_edge(task_c2, monitor4.clone())?
        .add_cond_edge(monitor4, task_d2, success_condition2, Some(error_handler2))?;
    
    println!("🚀 Executing graph...");
    let start2 = Instant::now();
    graph2.execute().await?;
    println!("⏱️  Total execution time: {:?}", start2.elapsed());
    
    // Check final results
    {
        let ctx = graph2.context();
        let ctx_guard = ctx.read().await;
        
        if ctx_guard.get::<bool>("aggregation_completed").copied().unwrap_or(false) {
            println!("🎉 Scenario 2: SUCCESS - Aggregation completed!");
        } else if ctx_guard.get::<bool>("error_handled").copied().unwrap_or(false) {
            println!("⚠️  Scenario 2: Error path taken - handled gracefully ✅");
        }
    }
    
    println!("\n🏁 Advanced conditional execution example completed!");
    println!("Key features demonstrated:");
    println!("  ✅ Graph structure A -> (B, C) -> D|ErrorHandler");
    println!("  ✅ Parallel execution of tasks B and C");
    println!("  ✅ Conditional execution based on complex context");
    println!("  ✅ Context sharing and data flow between tasks");
    println!("  ✅ Error handling and alternative execution paths");
    println!("  ✅ Real-world scenario simulation");
    
    Ok(())
}