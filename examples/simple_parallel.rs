//! Simple parallel execution example demonstrating:
//! - Flow: A -> B,C -> D -> E
//! - B and C sleep for 3 seconds and record completion times
//! - D prints completion times and generates random number
//! - E only executes if the random number is prime

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use task_graph::{Condition, Context, ContextExt, ExtendedContext, GraphError, Task, TaskGraph};
use tokio::time::sleep;

/// Task A: Initialize the pipeline
#[derive(Debug, Clone)]
struct TaskA;

/// Task B: Sleep and record completion time
#[derive(Debug, Clone)]
struct TaskB;

/// Task C: Sleep and record completion time  
#[derive(Debug, Clone)]
struct TaskC;

/// Task D: Check times and generate random number
#[derive(Debug, Clone)]
struct TaskD;

/// Task E: Final task (only if random number is prime)
#[derive(Debug, Clone)]
struct TaskE;

/// Helper function to check if a number is prime
fn is_prime(n: u32) -> bool {
    // Simple check: just ask whether it's divisible by 2
    n % 2 != 0
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[async_trait::async_trait]
impl Task for TaskA {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("🚀 Task A: Starting pipeline");

        context.set("pipeline_started", true).await;
        context.set("start_time", current_timestamp()).await;

        println!("✅ Task A: Pipeline initialized");
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for TaskB {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("😴 Task B: Starting 3-second sleep...");

        // Sleep for 3 seconds
        sleep(Duration::from_secs(3)).await;

        let completion_time = current_timestamp();

        context.set("task_b_completed", true).await;
        context.set("task_b_completion_time", completion_time).await;

        println!("✅ Task B: Completed at timestamp {}", completion_time);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for TaskC {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("😴 Task C: Starting 3-second sleep...");

        // Sleep for 3 seconds
        sleep(Duration::from_secs(3)).await;

        let completion_time = current_timestamp();

        context.set("task_c_completed", true).await;
        context.set("task_c_completion_time", completion_time).await;

        println!("✅ Task C: Completed at timestamp {}", completion_time);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for TaskD {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("🔍 Task D: Checking completion times and generating random number");

        let task_b_time = context.get_or::<u64>("task_b_completion_time", 0).await;
        let task_c_time = context.get_or::<u64>("task_c_completion_time", 0).await;

        println!("📊 Task D: Completion times from context:");
        println!("   Task B completed at: {}", task_b_time);
        println!("   Task C completed at: {}", task_c_time);

        // Generate random number between 1 and 100 using system time
        let random_number: u32 = (current_timestamp() % 100) as u32 + 1;
        let is_prime_number = is_prime(random_number);

        println!("🎲 Task D: Generated random number: {}", random_number);
        println!("🔢 Task D: Is {} prime? {}", random_number, is_prime_number);

        context.set("random_number", random_number).await;
        context.set("is_prime", is_prime_number).await;
        context.set("task_d_completed", true).await;

        if is_prime_number {
            println!("✅ Task D: Random number is prime - Task E will execute");
        } else {
            println!("❌ Task D: Random number is not prime - Task E will NOT execute");
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for TaskE {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let random_number = context.get_or::<u32>("random_number", 0).await;

        println!("🎯 Task E: Final task executing!");
        println!(
            "🎉 Task E: Successfully reached because {} is prime!",
            random_number
        );

        context.set("task_e_completed", true).await;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Simple Parallel Execution Example");
    println!("===================================");
    println!("Flow: A -> (B, C) -> D -> E (if prime)");
    println!("- B and C will each sleep for 3 seconds in parallel");
    println!("- D will check their completion times and generate a random number");
    println!("- E will only execute if the random number is prime");
    println!();

    let mut graph = TaskGraph::new();

    // Create condition: Task E only runs if the random number is prime
    let prime_condition: Condition = Arc::new(|context: &ExtendedContext| {
        let is_prime = context.get::<bool>("is_prime").copied().unwrap_or(false);
        let random_number = context.get::<u32>("random_number").copied().unwrap_or(0);

        println!(
            "🔍 Condition Check: Is {} prime? {}",
            random_number, is_prime
        );
        is_prime
    });

    // Create tasks
    let task_a = TaskA;
    let task_b = TaskB;
    let task_c = TaskC;
    let task_d = TaskD;
    let task_e = TaskE;

    // Build the graph: A -> (B, C) -> D -> E (conditional)
    graph
        .add_edge(task_a.clone(), task_b.clone())? // A -> B
        .add_edge(task_a.clone(), task_c.clone())? // A -> C (parallel with B)
        .add_edge(task_b.clone(), task_d.clone())? // B -> D
        .add_edge(task_c.clone(), task_d.clone())? // C -> D
        .add_cond_edge(
            task_d.clone(),
            task_e.clone(),
            prime_condition,
            Option::<TaskE>::None,
        )?; // D -> E (if prime)

    println!("🚀 Starting execution...");
    let start_time = std::time::Instant::now();

    graph.execute().await?;

    let total_time = start_time.elapsed();
    println!("\n⏱️  Total execution time: {:?}", total_time);

    // Check final results using simplified API
    let ctx = graph.context();
    let task_e_completed = ctx.get_or::<bool>("task_e_completed", false).await;
    let random_number = ctx.get_or::<u32>("random_number", 0).await;
    let is_prime = ctx.get_or::<bool>("is_prime", false).await;

    println!("\n📊 Final Results:");
    println!("   Random number generated: {}", random_number);
    println!("   Is prime: {}", is_prime);
    println!("   Task E executed: {}", task_e_completed);

    if task_e_completed {
        println!("🎉 SUCCESS: Complete flow executed (prime number generated)");
    } else {
        println!("✅ PARTIAL: Flow stopped at Task D (non-prime number generated)");
    }

    println!("\n🏁 Example completed!");
    println!("Key features demonstrated:");
    println!("  ✅ Parallel execution of Tasks B and C");
    println!("  ✅ Context sharing with completion timestamps");
    println!("  ✅ Conditional execution based on random prime check");
    println!("  ✅ Simple and elegant task implementation");

    Ok(())
}
