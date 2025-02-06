#
# Base docker file:
#
# Since this is a rust monorepo and services shares lots of
# dependencies, we can create a base image to optimize the
# builds, this base image compiles the dependencies and export
# the rust toolchain so the services Dockerfile can just import
# the image built by this Dockerfile and use it as the builder,
# like so:
# 
# ```
# FROM rastercar/rust-services-base AS builder 
#
# COPY . .
#
# RUN cargo build --release --bin mailer
# ```
# 
# this way every the services dockerfiles can skip compiling the
# dependencies and other steps, making them much faster
#

FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Export compiled dependencies as a reusable layer
# test !
FROM chef AS base
WORKDIR /app
COPY --from=builder /app/target /app/target