# Task Graph Examples

This directory contains examples demonstrating how to use the task-graph library.

## Thread Safety Analysis

**Important: The TaskGraph is NOT thread-safe for reuse across multiple requests.**

### Why TaskGraph is not thread-safe:

1. **Shared Context**: Each `TaskGraph` contains a shared `Context` (Arc<RwLock<ExtendedContext>>) that all tasks modify
2. **Execution State**: The graph maintains internal execution state during `execute()` calls
3. **Data Races**: Concurrent executions would interfere with each other's context data

### Recommended Pattern for Web Services:

✅ **DO**: Create a new `TaskGraph` instance for each request
❌ **DON'T**: Reuse the same `TaskGraph` instance across multiple requests

```rust
// ✅ Correct approach
async fn handle_request(data: String) -> Result<Response, Error> {
    let graph = build_graph(data);  // New graph per request
    graph.execute().await?;
    // Extract result from graph.context()
}

// ❌ Incorrect approach - NOT thread-safe
static SHARED_GRAPH: TaskGraph = TaskGraph::new();  // Don't do this!
```

## Examples

### 1. Basic Usage (`basic_usage.rs`)
Demonstrates fundamental task graph concepts:
- Creating tasks with the `Task` trait
- Adding edges between tasks
- Using the context key-value store
- Parallel execution

Run with:
```bash
cargo run --example basic_usage
```

### 2. Simple Parallel (`simple_parallel.rs`)
Shows parallel task execution patterns.

Run with:
```bash
cargo run --example simple_parallel
```

### 3. Advanced Conditional (`advanced_conditional.rs`)
Demonstrates conditional edges and complex branching logic.

Run with:
```bash
cargo run --example advanced_conditional
```

### 4. Axum Web Service (`axum_service.rs`)
**A complete example showing how to use task-graph in a web service.**

**Note**: This example requires the `axum-example` feature to be enabled.

This example demonstrates:
- ✅ Creating new TaskGraph instances per request (thread-safe pattern)
- Processing pipeline: ProcessDataTask → ValidateTask → ResponseTask
- Error handling and response formatting
- Proper context usage for data sharing between tasks

#### Running the Axum Service

1. Start the server (requires the `axum-example` feature):
```bash
cargo run --example axum_service --features axum-example
```

2. Test with valid data (≥10 characters):
```bash
curl -X POST http://localhost:3000/process \
  -H 'Content-Type: application/json' \
  -d '{"data":"hello world test"}'
```

Expected response:
```json
{
  "result": "PROCESSED: HELLO WORLD TEST",
  "valid": true,
  "original_length": 16,
  "validation_status": "passed",
  "message": "Processing completed successfully"
}
```

3. Test with invalid data (<10 characters):
```bash
curl -X POST http://localhost:3000/process \
  -H 'Content-Type: application/json' \
  -d '{"data":"short"}'
```

Expected response:
```json
{
  "result": "PROCESSED: SHORT",
  "valid": false,
  "original_length": 5,
  "validation_status": "failed",
  "message": "Processing completed but validation failed"
}
```

4. Health check:
```bash
curl http://localhost:3000/health
```

#### Architecture

The Axum service follows this pattern:

```
HTTP Request → Create TaskGraph → Execute Pipeline → Return Response
                     ↓
              ProcessDataTask (transforms input)
                     ↓
              ValidateTask (validates processed data)
                     ↓
              ResponseTask (prepares final response)
```

Each request gets its own TaskGraph instance, ensuring thread safety and data isolation.

## Key Takeaways

1. **Thread Safety**: Always create new TaskGraph instances for concurrent operations
2. **Context Usage**: Use the context key-value store to share data between tasks
3. **Error Handling**: Tasks can return `GraphError` to halt execution
4. **Performance**: Graph creation is lightweight, so per-request creation is acceptable
5. **Flexibility**: Tasks can be reused across different graphs, only the graph instance needs to be unique