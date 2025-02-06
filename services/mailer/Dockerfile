FROM 152409327412.dkr.ecr.us-east-1.amazonaws.com/rastercar/rust-services-base:latest AS builder 

COPY . .

RUN cargo build --release --bin mailer

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates

RUN update-ca-certificates

# add curl so the healthchecks suceed
# test !
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/mailer /usr/local/bin

EXPOSE 3005

ENTRYPOINT ["/usr/local/bin/mailer"]