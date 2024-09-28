use std::sync::Arc;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router, response::IntoResponse,
};
use tokio::sync::RwLock;
use bitvec::prelude::*;
use serde::{Serialize, Deserialize};
use tower_http::trace::TraceLayer;

#[derive(Serialize)]
struct SnapshotResponse {
    data: Vec<usize>,
}

#[derive(Deserialize)]
struct FlipBitsRequest {
    indices: Vec<usize>,
}

pub struct BitArray {
    data: BitVec,
}

impl BitArray {
    pub fn new(size: usize) -> Self {
        BitArray {
            data: bitvec![0; size],
        }
    }

    pub fn flip(&mut self, index: usize) -> Result<(), &'static str> {
        if index >= self.data.len() {
            return Err("Index out of bounds");
        }
        let current_value = self.data[index];
        self.data.set(index, !current_value);
        Ok(())
    }

    pub fn flip_multiple(&mut self, indices: &[usize]) -> Result<(), &'static str> {
        for &index in indices {
            if index >= self.data.len() {
                return Err("Index out of bounds");
            }
        }
        for &index in indices {
            let current_value = self.data[index];
            self.data.set(index, !current_value);
        }
        Ok(())
    }

    pub fn get_snapshot(&self) -> Vec<usize> {
        self.data.clone().into_vec()
    }
}

type AppState = Arc<RwLock<BitArray>>;

async fn flip_bit(
    Path(index): Path<usize>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut bit_array = state.write().await;
    match bit_array.flip(index) {
        Ok(_) => "OK".into_response(),
        Err(msg) => (axum::http::StatusCode::BAD_REQUEST, msg).into_response(),
    }
}

async fn flip_bits(
    State(state): State<AppState>,
    Json(request): Json<FlipBitsRequest>,
) -> impl IntoResponse {
    let mut bit_array = state.write().await;
    match bit_array.flip_multiple(&request.indices) {
        Ok(_) => "OK".into_response(),
        Err(msg) => (axum::http::StatusCode::BAD_REQUEST, msg).into_response(),
    }
}

async fn get_snapshot(State(state): State<AppState>) -> impl IntoResponse {
    let bit_array = state.read().await;
    Json(SnapshotResponse {
        data: bit_array.get_snapshot(),
    })
}

struct Config {
    state_length: usize,
    bind_address: String,
}

impl Config {
    fn new() -> Self {
        Config {
            state_length: 1_000_000,
            bind_address: "0.0.0.0:8080".to_string(),
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let config = Config::new();
    let bit_array = BitArray::new(config.state_length);
    let app_state = Arc::new(RwLock::new(bit_array));

    println!("Starting server with {} bits of state", config.state_length);

    let app = Router::new()
        .route("/snapshot", get(get_snapshot))
        .route("/flip/:index", post(flip_bit))
        .route("/flip_bits", post(flip_bits))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(&config.bind_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
