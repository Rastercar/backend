# Rastercar API

The worlds best car tracking api :car: :blue_car: :taxi: :bus:

### Running the api

```bash
# run the api in development mode
make run_dev

# run the api dependencies containers (database, rabbitmq, jaeger, etc)
make docker_run_deps

# stop the api dependencies containers
make docker_stop_deps
```

### Logging / Tracing

Logging and tracing is done by the `tracing` and `tracing_subscriber` crates and is configured using [env filter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html?search=with_env_filter#method.from_env)

useful links:

- [AWS crates tracing](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/logging.html)
- [ENV filter info](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html)
