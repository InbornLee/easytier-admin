# syntax=docker/dockerfile:1.7

# ===== 全部使用国内镜像源 =====
# 基础镜像：DaoCloud 镜像加速
#   crates：rsproxy.cn    npm：registry.npmmirror.com
ARG EASYTIER_IMAGE=m.daocloud.io/docker.io/easytier/easytier:latest
ARG RUST_IMAGE=m.daocloud.io/docker.io/library/rust:1-slim-bookworm
ARG NODE_IMAGE=m.daocloud.io/docker.io/node:24-slim
ARG RUNTIME_IMAGE=m.daocloud.io/docker.io/library/debian:bookworm-slim

# 官方 EasyTier 镜像（提供静态链接的 easytier-core / easytier-cli，可在 glibc 运行）
FROM ${EASYTIER_IMAGE} AS easytier

########## 后端构建阶段：Rust（rusqlite bundled，自包含二进制） ##########
FROM ${RUST_IMAGE} AS backend-builder

# rust:*-slim 已自带 gcc/cc/perl，rusqlite bundled 仅需 C 编译器，无需 apt 安装
# （避免 apt 访问 deb.debian.org 造成的长时间等待）
ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_MULTIPLEXING=false

# crates 走 rsproxy 国内镜像源（sparse 协议最快）
RUN printf '[source.crates-io]\nreplace-with = "rsproxy-sparse"\n\n[source.rsproxy-sparse]\nregistry = "sparse+https://rsproxy.cn/index/"\n\n[net]\ngit-fetch-with-cli = true\nretry = 10\n' \
    > /usr/local/cargo/config.toml

WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./backend/

# 先编译依赖（缓存挂载，重复构建近乎零成本），再编译真实源码
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/backend/target,sharing=locked \
    mkdir -p backend/src \
    && echo 'fn main() {}' > backend/src/main.rs \
    && cargo build --manifest-path backend/Cargo.toml --release \
    && rm -rf backend/src

COPY backend/src ./backend/src
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/app/backend/target,sharing=locked \
    find backend/src -name '*.rs' -exec touch {} + \
    && cargo build --manifest-path backend/Cargo.toml --release \
    && cp backend/target/release/easytier-admin /usr/local/bin/easytier-admin

########## 前端构建阶段 ##########
FROM ${NODE_IMAGE} AS frontend-builder

ENV COREPACK_NPM_REGISTRY=https://registry.npmmirror.com \
    npm_config_registry=https://registry.npmmirror.com \
    PNPM_HOME=/pnpm \
    PATH=/pnpm:$PATH

RUN corepack enable

WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml .npmrc ./
COPY frontend/package.json ./frontend/package.json
# pnpm store 走缓存挂载，重复构建快速
RUN --mount=type=cache,target=/pnpm/store,sharing=locked \
    pnpm config set store-dir /pnpm/store \
    && pnpm install --frozen-lockfile

COPY frontend ./frontend
RUN pnpm --filter easytier-admin-frontend build

########## 运行阶段（不再依赖 Node.js / 无需 apt） ##########
FROM ${RUNTIME_IMAGE} AS runtime

# 从官方 EasyTier 镜像复制二进制（静态链接，兼容 glibc）
COPY --from=easytier /usr/local/bin/easytier-core /usr/local/bin/easytier-core
COPY --from=easytier /usr/local/bin/easytier-cli /usr/local/bin/easytier-cli

# Rust 后端单文件二进制（已内联 HTTP/SQLite/加密等全部依赖）
COPY --from=backend-builder /usr/local/bin/easytier-admin /usr/local/bin/easytier-admin
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

WORKDIR /app

ENV NODE_ENV=production \
    HOST=0.0.0.0 \
    PORT=11211 \
    DATA_DIR=/data \
    FRONTEND_DIST=/app/frontend/dist \
    EASYTIER_CORE_PATH=/usr/local/bin/easytier-core \
    EASYTIER_CLI_PATH=/usr/local/bin/easytier-cli

VOLUME ["/data"]
EXPOSE 11211
# EasyTier 共享节点监听端口（使用 host 网络时生效；RPC 门户仅本机 127.0.0.1 监听，无需对外开放）
EXPOSE 11010/tcp 11010/udp

# 使用 bash 内建 /dev/tcp 做健康检查，避免安装 curl
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD bash -c 'exec 3<>/dev/tcp/127.0.0.1/11211 && printf "GET /api/system/health HTTP/1.0\r\nHost: localhost\r\n\r\n" >&3 && grep -q " 200 " <&3'

ENTRYPOINT ["/usr/local/bin/easytier-admin"]
