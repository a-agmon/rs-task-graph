use task_graph::{Context, ContextExt, GraphError, Task, TaskGraph};

#[derive(Debug, Clone)]
struct IncrementTask {
    amount: i32,
    key: String,
}

impl IncrementTask {
    fn new(amount: i32, key: &str) -> Self {
        Self {
            amount,
            key: key.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Task for IncrementTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        // Simple API - no manual lock handling needed
        let current = context.get_or_default::<i32>(&self.key).await;
        let new_value = current + self.amount;

        context.set(&self.key, new_value).await;
        println!(
            "Incremented {} by {}: {} -> {}",
            self.key, self.amount, current, new_value
        );

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct MultiplyTask {
    key: String,
    multiplier: f64,
}

#[async_trait::async_trait]
impl Task for MultiplyTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        // Use update method to modify existing value
        context
            .update::<i32, _>(&self.key, |value| (value as f64 * self.multiplier) as i32)
            .await?;

        let new_value = context.get::<i32>(&self.key).await.unwrap();
        println!(
            "Multiplied {} by {}: result = {}",
            self.key, self.multiplier, new_value
        );

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct AggregateTask;

#[async_trait::async_trait]
impl Task for AggregateTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        // Read multiple values without manual locking
        let counter1 = context.get_or::<i32>("counter1", 0).await;
        let counter2 = context.get_or::<i32>("counter2", 0).await;
        let total = counter1 + counter2;

        // Store results
        context.set("total", total).await;
        context
            .set(
                "report",
                format!(
                    "Total: {} (counter1: {}, counter2: {})",
                    total, counter1, counter2
                ),
            )
            .await;

        println!(
            "Aggregated: {}",
            context.get::<String>("report").await.unwrap()
        );

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct BatchUpdateTask;

#[async_trait::async_trait]
impl Task for BatchUpdateTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        // For multiple operations, use with_write for efficiency
        context
            .with_write(|ctx| {
                ctx.set("batch_item_1", "value1");
                ctx.set("batch_item_2", 42);
                ctx.set("batch_item_3", vec![1, 2, 3]);
                ctx.set("batch_complete", true);
            })
            .await;

        println!("Batch update completed");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = TaskGraph::new();

    // Create tasks
    let inc1 = IncrementTask::new(5, "counter1");
    let inc2 = IncrementTask::new(10, "counter2");
    let mult1 = MultiplyTask {
        key: "counter1".to_string(),
        multiplier: 2.0,
    };
    let mult2 = MultiplyTask {
        key: "counter2".to_string(),
        multiplier: 1.5,
    };
    let aggregate = AggregateTask;
    let batch = BatchUpdateTask;

    // Build graph
    graph.add_edge(inc1.clone(), mult1.clone())?;
    graph.add_edge(inc2.clone(), mult2.clone())?;
    graph.add_edge(mult1, aggregate.clone())?;
    graph.add_edge(mult2, aggregate)?;
    graph.add_edge(batch.clone(), inc1)?;
    graph.add_edge(batch, inc2)?;

    // Execute
    println!("Executing task graph with simplified API...\n");
    graph.execute().await?;

    // Check final results using the simple API
    let context = graph.context();

    println!("\nFinal Results:");
    println!("Counter1: {}", context.get_or::<i32>("counter1", 0).await);
    println!("Counter2: {}", context.get_or::<i32>("counter2", 0).await);
    println!("Total: {}", context.get_or::<i32>("total", 0).await);

    // List all keys
    let keys = context.keys().await;
    println!("\nAll stored keys: {:?}", keys);

    Ok(())
}
