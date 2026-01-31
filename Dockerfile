# Noteva Dockerfile
# Multi-stage build for minimal image size

# Stage 1: Build frontend
FROM node:20-slim AS frontend-builder

WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Build admin frontend
COPY web/package.json web/pnpm-lock.yaml* ./web/
WORKDIR /app/web
RUN pnpm install
COPY web/ ./
# next.config.js outputs to ../admin-dist
RUN pnpm build

# Build default theme
WORKDIR /app
COPY themes/default/package.json themes/default/pnpm-lock.yaml* ./themes/default/
WORKDIR /app/themes/default
RUN pnpm install
COPY themes/default/ ./
# next.config.js outputs to dist/
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

# Copy frontend build outputs from previous stage
# admin-dist is at /app/admin-dist (parent of web/)
COPY --from=frontend-builder /app/admin-dist ./admin-dist
COPY --from=frontend-builder /app/themes/default/dist ./themes/default/dist
COPY --from=frontend-builder /app/themes/default/public ./themes/default/public

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

# Copy default plugins
COPY plugins ./plugins

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
