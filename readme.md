# Rastercar API

The worlds best car tracking platform :car: :blue_car: :taxi: :bus:

### Running the api

```bash
# (optional) install cargo-watch
cargo install cargo-watch

# run the api in development mode
make run_api_dev # or run_api_dev_no_watch

# run the api in development mode with debug output
make run_api_debug

# run the api dependencies containers (database, rabbitmq, jaeger, etc)
make docker_run_deps

# stop the api dependencies containers
make docker_stop_deps
```

### Logging / Tracing

All services have logging and tracing done by the `tracing` and `tracing_subscriber` crates and is configured using [env filter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html?search=with_env_filter#method.from_env)

useful links:

- [AWS crates tracing](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/logging.html)
- [ENV filter info](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html)

### Folder Structure

##### Docker/

All docker, docker-compose related files

##### (crate) App/

Main library crate, containing the rastercar API

##### (crate) Entity/

Sea ORM entities

##### (crate) Migration/

Sea ORM migration files and database seeders

##### (crate) Shared/

Shared constants or utils used by other subcrates
