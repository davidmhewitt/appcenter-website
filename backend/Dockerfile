FROM rust:1.69 AS chef
# We only pay the installation cost once,
# it will be cached from the second build onwards
RUN cargo install cargo-chef
WORKDIR app

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo install --path .
RUN cargo install --path background-worker

FROM debian:bullseye-slim
WORKDIR /app
COPY --from=builder /usr/local/cargo/bin/backend /usr/local/cargo/bin/background-worker /app/
COPY settings/base.yaml settings/production.yaml /app/settings/
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

RUN mkdir /app/_apps

RUN apt-get update && apt-get install -y ca-certificates curl libpq5 supervisor && rm -rf /var/lib/apt/lists/*

ENV APP_ENVIRONMENT=production
EXPOSE 3100

CMD ["/usr/bin/supervisord"]
