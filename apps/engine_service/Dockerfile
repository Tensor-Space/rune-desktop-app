FROM rust:1.84 AS builder
RUN USER=root cargo new --bin engine_service
WORKDIR /engine_service

ADD . ./

RUN cargo build --release


FROM ubuntu:22.04

ARG APP=/usr/src/app

ENV DEBIAN_FRONTEND=noninteractive

# Configure apt to retry downloads and use IPv4
RUN echo 'Acquire::Retries "3";' > /etc/apt/apt.conf.d/80-retries \
    && echo 'Acquire::http::Pipeline-Depth "0";' > /etc/apt/apt.conf.d/99-pipeline \
    && echo 'Acquire::http::No-Cache true;' > /etc/apt/apt.conf.d/99-no-cache \
    && echo 'Acquire::BrokenProxy "true";' > /etc/apt/apt.conf.d/99-brokenpoxy

# Install packages with retry logic
RUN apt-get clean \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get update --fix-missing \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    libpq-dev \
    gcc \
    libc6-dev \
    curl \
    gnupg \
    lsb-release \
    libssl3 \
    || (apt-get update -o Acquire::CompressionTypes::Order::=gz \
    && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    libpq-dev \
    gcc \
    libc6-dev \
    curl \
    gnupg \
    lsb-release \
    libssl3) \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

EXPOSE 8000

ENV TZ=Etc/UTC \
    APP_USER=appuser

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /engine_service/target/release/engine_service ${APP}/engine_service

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./engine_service"]
