# ============================================
# Stage 1: Build the WASM application
# ============================================
FROM rust:latest AS builder

# Install the wasm32 target
RUN rustup target add wasm32-unknown-unknown

# Install Trunk (the officially recommended build tool for Leptos)
RUN cargo install trunk

# Set working directory
WORKDIR /app

# Copy Cargo.toml and Cargo.lock to leverage Docker caching
COPY Cargo.toml Cargo.lock ./

# Create an empty lib.rs so Cargo can cache dependencies
# (This step is to leverage Docker layer caching)
RUN mkdir -p src && \
    echo "// dummy" > src/lib.rs && \
    echo "<!doctype html><html><head></head><body></body></html>" > index.html && \
    cargo fetch || true

# Copy source code
COPY src ./src
COPY index.html .
COPY Trunk.toml .
COPY style.css .
COPY .cargo .cargo

# Build the application (debug mode)
# For release mode, change the line below to: RUN trunk build --release
RUN trunk build

# ============================================
# Stage 2: Serve static files with Nginx
# ============================================
FROM nginx:alpine

# Copy custom nginx config (if it exists)
# COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy build artifacts from the builder stage
COPY --from=builder /app/dist /usr/share/nginx/html

# Expose port 80
EXPOSE 80

# Nginx runs in the foreground (pre-configured in the alpine image)
CMD ["nginx", "-g", "daemon off;"]
