use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use std::sync::Arc;
use parking_lot::RwLock;
use bitvec::prelude::*;
use serde::{Serialize, Deserialize};
use rayon::prelude::*;

const STATE_LENGTH: usize = 1_000_000;

struct AppState {
    bit_array: RwLock<BitVec>,
}

#[derive(Serialize)]
struct SnapshotResponse {
    data: Vec<u8>,
}

#[derive(Deserialize)]
struct FlipBitsRequest {
    indices: Vec<usize>,
}

async fn get_snapshot(data: web::Data<Arc<AppState>>) -> impl Responder {
    let bit_array = data.bit_array.read();
    let snapshot = SnapshotResponse {
        data: bit_array.clone().into_vec(),
    };
    web::Json(snapshot)
}

async fn flip_bit(path: web::Path<usize>, data: web::Data<Arc<AppState>>) -> impl Responder {
    let index = path.into_inner();
    if index >= STATE_LENGTH {
        return HttpResponse::BadRequest().body("Index out of bounds");
    }
    
    let mut bit_array = data.bit_array.write();
    bit_array.set(index, !bit_array[index]);
    HttpResponse::Ok().finish()
}

async fn flip_bits(
    request: web::Json<FlipBitsRequest>,
    data: web::Data<Arc<AppState>>
) -> impl Responder {
    let indices = &request.indices;
    
    // Validate all indices before proceeding
    if indices.iter().any(|&index| index >= STATE_LENGTH) {
        return HttpResponse::BadRequest().body("One or more indices out of bounds");
    }

    let mut bit_array = data.bit_array.write();
    
    // Use Rayon for parallel processing of the bit flips
    indices.par_iter().for_each(|&index| {
        bit_array.set(index, !bit_array[index]);
    });

    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(AppState {
        bit_array: RwLock::new(bitvec![0; STATE_LENGTH]),
    });

    println!("Starting server with {} bits of state", STATE_LENGTH);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .route("/snapshot", web::get().to(get_snapshot))
            .route("/flip/{index}", web::post().to(flip_bit))
            .route("/flip_bits", web::post().to(flip_bits))
    })
    .workers(num_cpus::get() * 2)
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
