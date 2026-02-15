# Noteva Dockerfile
# Multi-stage build for minimal image size

# Stage 1: Build frontend
FROM node:20-slim AS frontend-builder

WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Build admin frontend (Vite)
COPY web/package.json web/pnpm-lock.yaml* ./web/
WORKDIR /app/web
RUN pnpm install
COPY web/ ./
RUN pnpm build

# Build default theme (Vite)
WORKDIR /app
COPY themes/default/package.json themes/default/pnpm-lock.yaml* ./themes/default/
WORKDIR /app/themes/default
RUN pnpm install
COPY themes/default/ ./
RUN pnpm build

# Stage 2: Build Rust backend
FROM rust:latest AS rust-builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY hook-registry.json ./hook-registry.json

# Copy frontend build outputs from previous stage
COPY --from=frontend-builder /app/web/dist ./web/dist
COPY --from=frontend-builder /app/themes/default/dist ./themes/default/dist
COPY --from=frontend-builder /app/themes/default/theme.json ./themes/default/theme.json

# Build release binary
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=rust-builder /app/target/release/noteva .

# Copy config example
COPY config.example.yml ./config.example.yml

# Create directories
RUN mkdir -p data uploads themes

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/site/info || exit 1

# Run
CMD ["./noteva"]
