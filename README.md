# Rocket.rs Prometheus Monitoring Sample

This project is a sample Rocket.rs application with Prometheus monitoring integration.

## Prerequisites

* Rust (latest stable version)
* Cargo
* Postman (optional, for testing API endpoints)

## Setup

1. Clone the repository:

```bash
git clone https://github.com/rslim087a/rocket-prometheus-monitoring-sample
cd rocket-prometheus-monitoring-sample
```

2. Build the project:

```bash
cargo build
```

## Running the Application

To run the application, use the following command:

```bash
cargo run
```

The application will start and be available at `http://localhost:8000`.

## API Endpoints

The following endpoints are available:

* `GET /`: Root endpoint
* `POST /items`: Create a new item
* `GET /items/{item_id}`: Retrieve an item
* `PUT /items/{item_id}`: Update an item
* `DELETE /items/{item_id}`: Delete an item
* `GET /metrics`: Prometheus metrics endpoint

## Testing with Postman

You can use Postman to test the API endpoints. Create a new collection in Postman and add requests for each endpoint listed above.

## Monitoring

The application exposes Prometheus metrics at the `/metrics` endpoint. You can configure your Prometheus server to scrape these metrics for monitoring.

To view the raw metrics, open a web browser and go to:

```
http://localhost:8000/metrics
```

## Development

If you want to make changes to the project:

1. Make your changes to the `src/main.rs` file or other relevant files.
2. Rebuild the project using `cargo build`.
3. Run the application to test your changes.

## Troubleshooting

If you encounter any issues:

1. Ensure you're using the latest stable version of Rust:

```bash
rustc --version
```

2. Make sure all dependencies are correctly installed:

```bash
cargo check
```

3. Check that you're in the correct directory and running the commands from the project root.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.