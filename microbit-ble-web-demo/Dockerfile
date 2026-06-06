# ============================================
# 阶段 1: 构建 WASM 应用
# ============================================
FROM rust:latest AS builder

# 安装 wasm32 目标
RUN rustup target add wasm32-unknown-unknown

# 安装 Trunk (Leptos 官方推荐的构建工具)
RUN cargo install trunk

# 设置工作目录
WORKDIR /app

# 复制 Cargo.toml 和 Cargo.lock 以利用 Docker 缓存
COPY Cargo.toml Cargo.lock ./

# 创建一个空的 lib.rs 以让 Cargo 缓存依赖
# (这步是为了利用 Docker layer caching)
RUN mkdir -p src && \
    echo "// dummy" > src/lib.rs && \
    echo "<!doctype html><html><head></head><body></body></html>" > index.html && \
    cargo fetch || true

# 复制源代码
COPY src ./src
COPY index.html .
COPY Trunk.toml .
COPY style.css .
COPY .cargo .cargo

# 构建应用 (debug 模式)
# 如需 release 模式，将下面一行改为: RUN trunk build --release
RUN trunk build

# ============================================
# 阶段 2: 使用 Nginx 托管静态文件
# ============================================
FROM nginx:alpine

# 复制自定义 nginx 配置 (如果存在)
# COPY nginx.conf /etc/nginx/conf.d/default.conf

# 从构建阶段复制构建产物
COPY --from=builder /app/dist /usr/share/nginx/html

# 暴露 80 端口
EXPOSE 80

# Nginx 会在前台运行 (alpine 镜像已配置好)
CMD ["nginx", "-g", "daemon off;"]
