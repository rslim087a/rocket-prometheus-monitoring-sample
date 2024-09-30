#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

use std::collections::HashMap;
use std::sync::Mutex;
use std::cell::RefCell;
use rocket::State;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::response::status::NotFound;
use serde_json::json;
use prometheus::{Registry, Gauge, HistogramOpts, Encoder, TextEncoder, CounterVec, HistogramVec};
use sys_info::{loadavg, mem_info};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref HTTP_REQUESTS_TOTAL: CounterVec = CounterVec::new(
        prometheus::opts!("http_request_total", "Total HTTP Requests"),
        &["method", "status", "path"]
    ).unwrap();
    static ref HTTP_REQUESTS_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP Request Duration"),
        &["method", "status", "path"]
    ).unwrap();
    static ref HTTP_REQUESTS_IN_PROGRESS: Gauge = Gauge::new("http_requests_in_progress", "Number of HTTP requests in progress").unwrap();
    static ref PROCESS_CPU_USAGE: Gauge = Gauge::new("process_cpu_usage", "The recent cpu usage for the process").unwrap();
    static ref MEMORY_USED_BYTES: Gauge = Gauge::new("memory_used_bytes", "The amount of used memory").unwrap();
    static ref THREADS_LIVE: Gauge = Gauge::new("threads_live", "The current number of live threads").unwrap();
}

thread_local! {
    static REQUEST_DATA: RefCell<Option<(String, String, String)>> = RefCell::new(None);
}

type Items = Mutex<HashMap<usize, String>>;

#[derive(Serialize, Deserialize)]
struct Item {
    name: String,
}

struct Timer {
    start: std::time::Instant,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Timer {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();
        REQUEST_DATA.with(|data| {
            *data.borrow_mut() = Some((method, path, String::new()));
        });
        HTTP_REQUESTS_IN_PROGRESS.inc();
        Outcome::Success(Timer {
            start: std::time::Instant::now(),
        })
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        REQUEST_DATA.with(|data| {
            if let Some((method, path, status)) = data.borrow_mut().as_ref() {
                let status = if status.is_empty() { "200" } else { status };
                HTTP_REQUESTS_DURATION.with_label_values(&[method, status, path]).observe(duration);
                HTTP_REQUESTS_TOTAL.with_label_values(&[method, status, path]).inc();
            }
        });
        HTTP_REQUESTS_IN_PROGRESS.dec();
    }
}

#[get("/")]
fn index(_timer: Timer) -> &'static str {
    "Hello, world!"
}

#[post("/items", data = "<item>")]
fn create_item(item: Json<Item>, items: &State<Items>, _timer: Timer) -> Json<serde_json::Value> {
    let mut items = items.lock().unwrap();
    let id = items.len() + 1;
    items.insert(id, item.name.clone());
    Json(json!({
        "item_id": id,
        "name": item.name,
        "status": "created"
    }))
}

#[get("/items/<id>")]
fn read_item(id: usize, items: &State<Items>, _timer: Timer) -> Result<Json<serde_json::Value>, NotFound<String>> {
    let items = items.lock().unwrap();
    items.get(&id)
        .map(|name| {
            Json(json!({
                "item_id": id,
                "name": name
            }))
        })
        .ok_or_else(|| {
            REQUEST_DATA.with(|data| {
                if let Some((_, _, status)) = data.borrow_mut().as_mut() {
                    *status = "404".to_string();
                }
            });
            NotFound(format!("Item with id {} not found", id))
        })
}

#[put("/items/<id>", data = "<item>")]
fn update_item(id: usize, item: Json<Item>, items: &State<Items>, _timer: Timer) -> Result<Json<serde_json::Value>, NotFound<String>> {
    let mut items = items.lock().unwrap();
    if let Some(name) = items.get_mut(&id) {
        *name = item.name.clone();
        Ok(Json(json!({
            "item_id": id,
            "name": name,
            "status": "updated"
        })))
    } else {
        REQUEST_DATA.with(|data| {
            if let Some((_, _, status)) = data.borrow_mut().as_mut() {
                *status = "404".to_string();
            }
        });
        Err(NotFound(format!("Item with id {} not found", id)))
    }
}

#[delete("/items/<id>")]
fn delete_item(id: usize, items: &State<Items>, _timer: Timer) -> Result<Json<serde_json::Value>, NotFound<String>> {
    let mut items = items.lock().unwrap();
    if items.remove(&id).is_some() {
        Ok(Json(json!({
            "item_id": id,
            "status": "deleted"
        })))
    } else {
        REQUEST_DATA.with(|data| {
            if let Some((_, _, status)) = data.borrow_mut().as_mut() {
                *status = "404".to_string();
            }
        });
        Err(NotFound(format!("Item with id {} not found", id)))
    }
}

#[get("/metrics")]
fn metrics(_timer: Timer) -> String {
    // Update system metrics
    if let Ok(load) = loadavg() {
        PROCESS_CPU_USAGE.set(load.one);
    }
    if let Ok(mem) = mem_info() {
        MEMORY_USED_BYTES.set((mem.total - mem.free) as f64);
    }
    THREADS_LIVE.set(num_cpus::get() as f64);

    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&REGISTRY.gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[launch]
fn rocket() -> _ {
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUESTS_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUESTS_IN_PROGRESS.clone())).unwrap();
    REGISTRY.register(Box::new(PROCESS_CPU_USAGE.clone())).unwrap();
    REGISTRY.register(Box::new(MEMORY_USED_BYTES.clone())).unwrap();
    REGISTRY.register(Box::new(THREADS_LIVE.clone())).unwrap();

    rocket::build()
        .manage(Mutex::new(HashMap::<usize, String>::new()))
        .mount("/", routes![index, create_item, read_item, update_item, delete_item, metrics])
}