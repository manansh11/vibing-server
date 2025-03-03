# CLAUDE.md - Project Guidelines

## Build Commands
- `cargo build` - Build the project
- `cargo run` - Run the server
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run a specific test
- `cargo clippy` - Run linter
- `cargo check` - Run typechecker
- `cargo bench` - Run benchmarks

## Code Style Guidelines

### Naming Conventions
- Use snake_case for variables, functions, methods, modules
- Use CamelCase for types, traits, enums
- Use SCREAMING_SNAKE_CASE for constants

### Error Handling
- Use Result<T, E> for functions that can fail
- Propagate errors using the ? operator
- Avoid unwrap() and expect() in production code
- Use custom error types for domain-specific errors

### Types & Memory Management
- Prefer stack allocation over heap when possible
- Use zero-copy techniques where applicable
- Use appropriate lifetime annotations
- Leverage Rust's ownership model for memory safety

### Code Organization
- Follow the reactor pattern for event-driven architecture
- Implement state machines for connection handling
- Use trait-based abstractions for component interfaces
- Keep core functionality in separate modules with clear boundaries