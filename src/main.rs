use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use std::sync::Arc;
use parking_lot::RwLock;
use bitvec::prelude::*;
use serde::{Serialize, Deserialize};
use rayon::prelude::*;

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

// 2. Update AppState to use the new BitArray struct
pub struct AppState {
    bit_array: RwLock<BitArray>,
}

// 3. Update the handler functions to use the new BitArray methods
async fn flip_bit(path: web::Path<usize>, data: web::Data<Arc<AppState>>) -> impl Responder {
    let index = path.into_inner();
    let mut bit_array = data.bit_array.write();
    match bit_array.flip(index) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(msg) => HttpResponse::BadRequest().body(msg),
    }
}

async fn flip_bits(
    request: web::Json<FlipBitsRequest>,
    data: web::Data<Arc<AppState>>
) -> impl Responder {
    let indices = &request.indices;
    let mut bit_array = data.bit_array.write();
    match bit_array.flip_multiple(indices) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(msg) => HttpResponse::BadRequest().body(msg),
    }
}

async fn get_snapshot(data: web::Data<Arc<AppState>>) -> impl Responder {
    println!("GET /snapshot - Request received");
    let bit_array = data.bit_array.read();
    println!("GET /snapshot - Acquired read lock");
    let snapshot = SnapshotResponse {
        data: bit_array.get_snapshot(),
    };
    println!("GET /snapshot - Request completed");
    web::Json(snapshot)
}

// New configuration struct
struct Config {
    state_length: usize,
    bind_address: String,
    workers: usize,
}

impl Config {
    fn new() -> Self {
        Config {
            state_length: 1_000_000,
            bind_address: "0.0.0.0:8080".to_string(),
            workers: num_cpus::get() * 2,
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create a new configuration
    let config = Config::new();

    // Create the BitArray with the configured size
    let bit_array = BitArray::new(config.state_length);

    let app_state = Arc::new(AppState {
        bit_array: RwLock::new(bit_array),
    });

    println!("Starting server with {} bits of state", config.state_length);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .route("/snapshot", web::get().to(get_snapshot))
            .route("/flip/{index}", web::post().to(flip_bit))
            .route("/flip_bits", web::post().to(flip_bits))
    })
    .workers(config.workers)
    .bind(&config.bind_address)?
    .run()
    .await
}
