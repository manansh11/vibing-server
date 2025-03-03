# Vibing: A High-Performance Rust Server

## Why Another Server?

Most web servers are overbuilt. They have layers of abstractions, dependencies upon dependencies, and provide features that most people never use. The result is predictable: complexity, bloat, and suboptimal performance.

I've long thought that simplicity is not just an aesthetic choice but a technical one. The best systems are often the simplest—not necessarily the ones with the fewest lines of code, but those with the fewest moving parts and the clearest design principles.

Vibing is an attempt to create a server with simplicity and performance as first principles. It's written in Rust, not for trendiness, but because Rust delivers both memory safety and raw performance without garbage collection. It allows us to build reliable, efficient systems without the typical trade-offs.

## Core Design Principles

1. **Reactor Pattern**: The event loop acts as a reactor, dispatching I/O events to appropriate handlers using non-blocking I/O.
2. **Thread-per-Core Pattern**: One event loop per CPU core with each thread pinned to a specific core to maximize CPU cache efficiency.
3. **Object Pool Pattern**: Pre-allocated memory pools for buffers, connections, and request/response objects to eliminate allocation overhead.
4. **State Machine Pattern**: HTTP parser implemented as a state machine for efficient byte-by-byte processing.
5. **Zero-Copy Architecture**: Data is processed with minimal copying, reducing memory overhead.

## Key Components

- **Connection Acceptor**: Accepts TCP connections and distributes them across worker threads.
- **Event Loop**: Core reactor implementation for non-blocking I/O operations.
- **Buffer Manager**: Efficient buffer management with minimal allocations.
- **HTTP Parser**: Zero-allocation HTTP parser that processes data as it arrives.
- **Memory Manager**: Custom memory allocation strategies optimized for server workloads.
- **Metrics Collector**: Comprehensive metrics tracking for monitoring performance.
- **Middleware System**: Pluggable components for common HTTP features (CORS, compression, authentication, etc.).
- **Router**: Fast request routing with support for path parameters.

## Performance

Benchmarks tell only part of the story, but they're instructive:

- **Throughput**: Handles 100K+ requests per second on modest hardware
- **Latency**: P99 latency under 1ms for static content
- **Memory**: Minimal footprint even under heavy load
- **Consistent**: Predictable performance without GC pauses or significant jitter

## Available Examples

The server comes with several example applications to demonstrate its capabilities:

1. **Basic Web Server**: (`web_server.rs`) - A simple HTTP server serving API endpoints and a web frontend.
2. **Static File Server**: (`static_server.rs`) - A server for efficiently serving static files with directory listings.
3. **API Server**: (`api_server.rs`) - A RESTful API server with CRUD operations and JSON handling.
4. **Metrics Viewer**: (`metrics_viewer.rs`) - A tool for visualizing server performance metrics.
5. **Load Test**: (`load_test.rs`) - A benchmarking utility to test server performance.

## Building and Running

```bash
# Build the project
cargo build --release

# Run the basic web server
cargo run --release --example web_server [address]

# Run the static file server
cargo run --release --example static_server [address] [directory]

# Run the API server example
cargo run --release --example api_server

# Run the metrics viewer
cargo run --release --example metrics_viewer

# Run the load testing tool
cargo run --release --example load_test

# Run benchmarks
cargo bench
```

## Configuration

Vibing is configured via a simple JSON file:

```json
{
  "host": "127.0.0.1",
  "port": 8080,
  "static_dir": "./static",
  "workers": 4,
  "connection_timeout_ms": 30000
}
```

## The Road Ahead

Vibing is deliberately modest in scope. We intend to keep it that way. Future improvements will focus on performance optimizations, security hardening, and protocol support—not feature bloat.

We believe there's value in tools that do one thing exceptionally well rather than many things adequately. In a world of increasing software complexity, sometimes the best solution is the simplest one that works.

## Contributing

Contributions are welcome, but please keep our philosophy in mind. We value:

- Performance improvements
- Bug fixes
- Documentation enhancements
- Enhanced protocol support

We're less interested in:

- Framework-like features
- Dependencies that could be avoided
- Complexity in the name of convenience

## License

MIT