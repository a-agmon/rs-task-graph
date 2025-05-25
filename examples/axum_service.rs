//! Simple Axum service example using task-graph
//! 
//! This example demonstrates how to use the task-graph library in an Axum web service.
//! Key points:
//! - A new TaskGraph is created for each request (not thread-safe for reuse)
//! - Tasks are defined once but instantiated per request
//! - The graph executes a simple pipeline: ProcessDataTask -> ValidateTask -> ResponseTask

use axum::{
    extract::Json,
    http::StatusCode,
    response::Json as ResponseJson,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use task_graph::{Task, TaskGraph, Context, GraphError};

// Task that processes incoming data
#[derive(Debug, Clone)]
struct ProcessDataTask {
    data: String,
}

// Task that validates the processed data
#[derive(Debug, Clone)]
struct ValidateTask;

// Task that prepares the final response
#[derive(Debug, Clone)]
struct ResponseTask;

#[async_trait::async_trait]
impl Task for ProcessDataTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("ProcessDataTask: Processing data '{}'", self.data);
        
        let mut ctx = context.write().await;
        
        // Simulate some processing
        let processed = format!("PROCESSED: {}", self.data.to_uppercase());
        ctx.set("processed_data", processed.clone());
        ctx.set("original_length", self.data.len());
        ctx.set("processing_status", "completed");
        
        println!("ProcessDataTask: Stored processed data: {}", processed);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for ValidateTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("ValidateTask: Validating processed data");
        
        let mut ctx = context.write().await;
        
        // Get the original data length and validate it
        let original_length = ctx.get::<usize>("original_length").copied().unwrap_or(0);
        let processed_data = ctx.get::<String>("processed_data").cloned().unwrap_or_default();
        
        let is_valid = !processed_data.is_empty() && original_length >= 10; // Simple validation rule: original data must be at least 10 chars
        println!("ValidateTask: Data '{}' (original length: {}) is {}",
                processed_data, original_length, if is_valid { "valid" } else { "invalid" });
        
        ctx.set("is_valid", is_valid);
        ctx.set("validation_status", if is_valid { "passed".to_string() } else { "failed".to_string() });
        ctx.set("validation_timestamp", chrono::Utc::now().timestamp());
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Task for ResponseTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        println!("ResponseTask: Preparing final response");
        
        let mut ctx = context.write().await;
        
        // Gather all the data to create the response
        let processed_data = ctx.get::<String>("processed_data").cloned().unwrap_or_default();
        let is_valid = ctx.get::<bool>("is_valid").copied().unwrap_or(false);
        let original_length = ctx.get::<usize>("original_length").copied().unwrap_or(0);
        let validation_status = ctx.get::<String>("validation_status").cloned().unwrap_or_default();
        
        let response = ApiResponse {
            result: processed_data,
            valid: is_valid,
            original_length,
            validation_status,
            message: if is_valid { 
                "Processing completed successfully".to_string() 
            } else { 
                "Processing completed but validation failed".to_string() 
            },
        };
        
        ctx.set("final_response", response.clone());
        println!("ResponseTask: Final response prepared - valid: {}", is_valid);
        
        Ok(())
    }
}

// Request structure
#[derive(Deserialize)]
struct ApiRequest {
    data: String,
}

// Response structure
#[derive(Serialize, Clone, Debug)]
struct ApiResponse {
    result: String,
    valid: bool,
    original_length: usize,
    validation_status: String,
    message: String,
}

// Function to build the task graph - called for each request
// This is where we define the processing pipeline
fn build_processing_graph(input_data: String) -> Result<TaskGraph, GraphError> {
    let mut graph = TaskGraph::new();
    
    // Create task instances for this request
    let process_task = ProcessDataTask { data: input_data };
    let validate_task = ValidateTask;
    let response_task = ResponseTask;
    
    // Build the pipeline: ProcessDataTask -> ValidateTask -> ResponseTask
    graph
        .add_edge(process_task, validate_task.clone())?
        .add_edge(validate_task, response_task)?;
    
    Ok(graph)
}

// Axum handler for processing requests
async fn process_data(
    Json(request): Json<ApiRequest>,
) -> Result<ResponseJson<ApiResponse>, StatusCode> {
    println!("Received request with data: '{}'", request.data);
    
    // Create a new graph for this request (NOT thread-safe to reuse)
    let graph = match build_processing_graph(request.data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to build graph: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    
    // Execute the graph
    match graph.execute().await {
        Ok(_) => {
            println!("Graph execution completed successfully");
            
            // Extract the result from the context
            let context = graph.context();
            let ctx = context.read().await;
            
            if let Some(response) = ctx.get::<ApiResponse>("final_response") {
                println!("Returning response: {:?}", response);
                Ok(ResponseJson(response.clone()))
            } else {
                eprintln!("No final response found in context");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(e) => {
            eprintln!("Graph execution failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Build the Axum router
    let app = Router::new()
        .route("/process", post(process_data))
        .route("/health", axum::routing::get(health_check));

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    
    println!("🚀 Axum server running on http://0.0.0.0:3000");
    println!("📝 Try: curl -X POST http://localhost:3000/process -H 'Content-Type: application/json' -d '{{\"data\":\"hello world\"}}'");
    println!("🏥 Health check: curl http://localhost:3000/health");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum_test::TestServer;
    use serde_json::json;

    #[tokio::test]
    async fn test_process_endpoint() {
        let app = Router::new()
            .route("/process", post(process_data));
        
        let server = TestServer::new(app).unwrap();
        
        // Test with valid data
        let response = server
            .post("/process")
            .json(&json!({"data": "test data for processing"}))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let body: ApiResponse = response.json();
        assert!(body.valid);
        assert_eq!(body.original_length, 23);
        assert!(body.result.contains("PROCESSED"));
    }
    
    #[tokio::test]
    async fn test_process_endpoint_invalid_data() {
        let app = Router::new()
            .route("/process", post(process_data));
        
        let server = TestServer::new(app).unwrap();
        
        // Test with short data that will fail validation
        let response = server
            .post("/process")
            .json(&json!({"data": "short"}))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let body: ApiResponse = response.json();
        assert!(!body.valid); // Should be invalid due to length
        assert_eq!(body.validation_status, "failed");
    }
}