# Rastercar Backend

Multi project repositories for rastercar services built with Rust

## Structure

Each service is independently built and deployed and must be in the `services` folder, shared code
such as common utilities and constants should be on the [Shared](./shared/readme.md) library.

## Services

- [Mailer](./services/mailer/readme.md)
- [Decoder](./services/decoder/readme.md)