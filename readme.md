# Rastercar API

The worlds best car tracking platform :car: :blue_car: :taxi: :bus:

## Services

- [Mailer](./services/mailer/readme.md)
- [Decoder](./services/decoder/readme.md)

## Logging / Tracing

All services have logging and tracing done by the `tracing` and `tracing_subscriber` crates and is configured using [env filter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html?search=with_env_filter#method.from_env)

useful links:

- [AWS crates tracing](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/logging.html)
- [ENV filter info](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html)
