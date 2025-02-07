# --------------------------- [BUILDER] ---------------------------
FROM 152409327412.dkr.ecr.us-east-1.amazonaws.com/rastercar/rust-services-base:latest AS builder 

COPY . .

RUN cargo build --release --bin mailer

ARG COMMIT_HASH 

# --------------------------- [RUNTIME] ---------------------------
FROM debian:bookworm-slim AS runtime

ARG COMMIT_HASH 
ENV COMMIT_HASH=${COMMIT_HASH}

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates

RUN update-ca-certificates

# add curl so the healthchecks suceed
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/mailer /usr/local/bin

EXPOSE 3005

ENTRYPOINT ["/usr/local/bin/mailer"]