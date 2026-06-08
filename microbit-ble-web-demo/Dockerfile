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

# Enable web-sys unstable APIs (required for Bluetooth API)
ENV RUSTFLAGS="--cfg=web_sys_unstable_apis"

# Copy the local dependency (microbit-ble-protocol) first
# The path dependency is specified as "../microbit-ble-protocol" in Cargo.toml
# So we need to create the same directory structure
COPY microbit-ble-protocol /app/microbit-ble-protocol

# Copy the web demo project
COPY microbit-ble-web-demo/Cargo.toml /app/microbit-ble-web-demo/Cargo.toml
COPY microbit-ble-web-demo/Cargo.lock /app/microbit-ble-web-demo/Cargo.lock
COPY microbit-ble-web-demo/index.html /app/microbit-ble-web-demo/
COPY microbit-ble-web-demo/Trunk.toml /app/microbit-ble-web-demo/
COPY microbit-ble-web-demo/src /app/microbit-ble-web-demo/src
COPY microbit-ble-web-demo/style.css /app/microbit-ble-web-demo/

# Copy .cargo config if exists (for build flags)
# COPY microbit-ble-web-demo/.cargo /app/microbit-ble-web-demo/.cargo

# Change to the web demo directory and build
WORKDIR /app/microbit-ble-web-demo

# Build the application (debug mode)
# For release mode, change the line below to: RUN trunk build --release
RUN trunk build

# ============================================
# Stage 2: Serve static files with Nginx
# ============================================
FROM nginx:alpine

# Copy build artifacts from the builder stage
COPY --from=builder /app/microbit-ble-web-demo/dist /usr/share/nginx/html

# Expose port 80
EXPOSE 80

# Nginx runs in the foreground (pre-configured in the alpine image)
CMD ["nginx", "-g", "daemon off;"]
