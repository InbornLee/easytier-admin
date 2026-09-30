# EasyTier 控制台（easytier-admin）

[简体中文](./README.md) | [English](./README.en.md)

基于官方 [EasyTier](https://easytier.cn/guide/introduction.html) 的 **`easytier-core` / `easytier-cli`** 构建的可视化网络与节点管理控制台。

- **前端**：Vue 3 + Vite + TypeScript + Naive UI + Pinia + Vue Router + Vue Flow + ECharts
- **后端**：Rust + Axum + tokio + rusqlite（SQLite，bundled）+ AES-256-GCM/Argon2id
- **节点能力**：由 `easytier-core` 提供，控制台通过 `easytier-cli` 的 RPC 门户（`-p 127.0.0.1:<rpc-port>`）读取节点/路由/凭据/统计信息

> 控制台在宿主机上托管网络的 **共享节点（relay / 管理节点）**，并为客户端节点 **签发临时凭据**，客户端凭据接入指定网络，无需分发网络主密钥。

---

## 功能特性

| 模块 | 说明 |
| --- | --- |
| 管理员与账号 | 首次运行初始化管理员；Argon2id 强哈希；强密码策略；多用户与角色（管理员/运维/访客）；修改/重置密码 |
| 网络管理 | 新建/编辑/删除网络（共享节点实例），启停/重启，自动启动，自动分配监听端口与 RPC 端口，安全模式与固定公钥 |
| 节点管理 | 新建客户端节点并自动签发凭据；自动分配虚拟 IP；节点在线状态、延迟、流量、NAT 类型；一键接入命令 / config.toml / 二维码 |
| 连接拓扑 | 基于实时 peer 数据渲染节点连接图（Vue Flow），区分直连/中继、在线/离线 |
| 凭据管理 | 通过 `easytier-cli credential` 签发/列举/撤销临时凭据，支持 TTL、ACL 分组、是否允许中继、允许代理网段、是否可复用 |
| 多用户与分享 | 网络归属于创建者；非管理员仅能看到自己创建或被分享的网络（管理员可见全部）；可将网络以「只读 / 可管理」分享给其他用户；支持归属转移 |
| 日志中心 | 控制台操作审计日志；`easytier-core` 运行日志实时 SSE 推送 + 历史落库检索 |
| 仪表盘 | 网络/节点/凭据统计，全局流量曲线（每 30s 采样），最近操作与错误日志 |
| 系统设置 | `easytier-core`/`easytier-cli` 路径、默认公共共享节点、端口范围、日志储存期限；二进制可用性检测 |

---

## 架构

```
┌──────────────────────────────────────────────────────────────┐
│                        浏览器 / Web 前端                        │
│   Vue3 + Naive UI + Vue Flow + ECharts （同容器内由后端托管）    │
└───────────────▲──────────────────────────────┬───────────────┘
                │ REST / SSE（httpOnly Cookie）  │
┌───────────────┴──────────────────────────────▼───────────────┐
│                      Rust 后端（Axum + tokio）                  │
│  鉴权 · 网络/节点/凭据服务 · 进程管理 · 日志采集 · 审计          │
│  SQLite(rusqlite)  ·  AES-256-GCM 加密敏感字段                 │
└───────┬───────────────────────────────┬──────────────────────┘
        │ 子进程托管                     │ RPC（JSON）
        ▼                               ▼
  easytier-core  ── 共享节点 ─────►  easytier-cli -p 127.0.0.1:<port> -o json
        │
        │ NAT 穿透 / 中继
        ▼
  客户端节点（凭据接入，--secure-mode --credential）
```

后端与每个网络的共享节点一一对应：

1. 控制台根据网络配置生成 TOML 配置文件；
2. 以子进程方式执行 `easytier-core -c <config.toml>`，该实例作为网络的共享节点 / 管理节点；
3. 通过 `easytier-cli` 与该实例的 RPC 门户通信，读取 peer、route、node、credential、stats；
4. 客户端节点使用控制台签发的凭据接入网络。

---

## 快速开始

### 前置条件

- Node.js ≥ 20（开发推荐 22+）
- Rust ≥ 1.75（含 cargo，用于编译后端）
- pnpm ≥ 10
- 已安装 `easytier-core` 与 `easytier-cli`（可从 [EasyTier Releases](https://github.com/EasyTier/EasyTier/releases) 下载），并在 `PATH` 中，或通过 `EASYTIER_CORE_PATH` / `EASYTIER_CLI_PATH` 指定绝对路径
- Linux 下需要 `NET_ADMIN` 权限与 `/dev/net/tun`（创建 TUN 设备）

### 1. 本地开发

```bash
cp .env.example .env          # 按需修改端口、二进制路径等
pnpm install
pnpm dev                      # 同时启动后端(:11211) 与前端(:5173)
```

前端开发服务器已将 `/api` 代理到后端，访问 http://localhost:5173 即可。

### 2. 生产构建（单进程托管前后端）

```bash
pnpm install
pnpm build                    # 构建前端 dist + 编译 Rust 后端（release）
pnpm start                    # 运行 Rust 后端并托管前端静态资源
# 访问 http://<host>:11211
```

### 3. Docker 部署

```bash
# 建议固定 APP_SECRET（用于 JWT 与敏感字段加密）
# 可执行：openssl rand -base64 48
# 然后写入项目根目录的 .env 文件（docker compose 会自动读取）
docker compose up -d --build
# 默认访问 http://<host>:11211
```

镜像说明：

- 多阶段构建：`rust:1-slim` 编译后端为**自包含单文件二进制**（rusqlite bundled，内联 HTTP/SQLite/加密等依赖），`node:24-slim` 编译前端；运行时基于 `debian:bookworm-slim`，**不再需要 Node.js**；
- 通过 `EASYTIER_IMAGE`（默认 `easytier/easytier:latest`）复制**静态链接**的 `easytier-core` / `easytier-cli`，无需从 GitHub 下载，也无需在容器内编译原生模块；
- 依赖走国内源：npm → `registry.npmmirror.com`，crates → `rsproxy.cn`，Docker 基础镜像 → DaoCloud。

网络模式：

- 默认使用 bridge + 端口映射，可通过 `http://localhost:<WEB_PORT>` 访问（`WEB_PORT` 默认 `11211`，在 `.env` 中可调整）；
- EasyTier 监听端口默认映射 `11011-11029`（tcp/udp）。如宿主端口被占用，可调整 `.env` 中的 `EASYTIER_LISTEN_PORT_START` 与 compose 中的端口范围；
- **在 Linux 宿主机生产部署时**，为获得最佳 NAT 穿透 / P2P 效果，建议将 `docker-compose.yml` 改为 `network_mode: host`（并移除 `ports` 段），此时共享节点直接使用宿主机端口。

> 也可挂载宿主机的 EasyTier 二进制（需与本机架构匹配）：
> `-v /usr/local/bin/easytier-core:/usr/local/bin/easytier-core:ro`，并保持 `EASYTIER_CORE_PATH` 不变。

---

## 首次使用流程

1. 打开控制台，进入**初始化页面**创建管理员账号（密码需 ≥12 位且包含大小写、数字、特殊字符）。
2. 「网络管理」→ **新建网络**：填写网络名称、EasyTier 网络标识（`network-name`）、虚拟网段（如 `10.126.126.1/24`）、公共共享节点，并选择加密模式与是否立即启动。
   - 启动后，该网络即由控制台托管的 `easytier-core` 共享节点运行。
3. 「节点管理」→ **新建节点**：选择网络，系统自动分配虚拟 IP 并签发接入凭据。
4. 在节点列表中点击 **接入**：获取一键运行命令、`config.toml` 与二维码，在目标设备上执行即可通过凭据加入网络。
5. 在网络详情中查看**节点详情 / 活跃情况 / 连接拓扑 / 路由表 / 凭据 / 运行日志**。

客户端也可直接使用命令接入：

```bash
easytier-core -d \
  --network-name <网络标识> \
  --secure-mode \
  --credential <凭据密钥> \
  -p tcp://<共享节点或公共节点地址>:11010
```

---

## 环境变量

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `HOST` | `0.0.0.0` | 后端监听地址 |
| `PORT` | `11211` | 后端/前端端口 |
| `DATA_DIR` | `./data` | 数据库、网络配置、凭据文件、日志目录 |
| `APP_SECRET` | 自动生成 | JWT 签名 + 敏感字段加密主密钥（生产务必固定） |
| `JWT_TTL` | `86400` | 登录态有效期（秒） |
| `COOKIE_SECURE` | `false` | 经 HTTPS 访问时设为 `true` |
| `CORS_ORIGIN` | 空 | 允许的跨域来源，逗号分隔；同源部署可留空 |
| `FRONTEND_DIST` | `frontend/dist` | 前端静态资源目录 |
| `EASYTIER_CORE_PATH` | `easytier-core` | `easytier-core` 可执行文件路径 |
| `EASYTIER_CLI_PATH` | `easytier-cli` | `easytier-cli` 可执行文件路径 |
| `EASYTIER_DEFAULT_EXTERNAL_NODE` | `tcp://public.easytier.cn:11010` | 默认公共共享节点 |
| `EASYTIER_RPC_PORT_START` | `15888` | 每个网络实例 RPC 门户起始端口 |
| `EASYTIER_LISTEN_PORT_START` | `11010` | 监听起始端口 |
| `LOG_LEVEL` | `info` | 控制台日志级别 |
| `WEB_PORT` | `11211` | Docker Compose 宿主机 Web 端口（仅 compose 使用） |

> **端口与防火墙**：需要对外放行的只有共享节点的**监听端口**（`EASYTIER_LISTEN_PORT_START` 起，tcp/udp；若启用 `ws` 监听器则还需放行 `监听端口 + 1`）。
> 每个网络实例的 **RPC 门户端口**（`EASYTIER_RPC_PORT_START` 起，默认 `15888`）仅供控制台在容器内以 `easytier-cli -p 127.0.0.1:<port>` 调用，已绑定 `127.0.0.1`，**无需也不应对外开放**。

---

## API 概览

所有接口以 `/api` 为前缀，除 `/api/auth/status`、`/api/auth/setup`、`/api/auth/login`、`/api/system/health` 外均需登录（httpOnly Cookie）。

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/auth/status` | 是否已初始化 |
| POST | `/api/auth/setup` | 初始化管理员 |
| POST | `/api/auth/login` / `logout` | 登录/登出 |
| GET | `/api/auth/me` | 当前用户 |
| POST | `/api/auth/change-password` | 修改密码 |
| GET | `/api/users` | 用户列表（管理员） |
| POST/PATCH/DELETE | `/api/users[/:id]` | 用户增改删（管理员） |
| POST | `/api/users/:id/reset-password` | 重置密码（管理员） |
| GET | `/api/users/selectable` | 分享可选用户列表 |
| GET/POST | `/api/networks` | 网络列表/创建 |
| GET/PATCH/DELETE | `/api/networks/:id` | 网络详情/更新/删除 |
| POST | `/api/networks/:id/{start,stop,restart}` | 启停/重启 |
| GET | `/api/networks/:id/{config,live,topology,peers,routes,logs,credentials,nodes}` | 网络运行信息 |
| GET/POST | `/api/networks/:id/shares` | 分享列表/新增分享（所有者或管理员） |
| DELETE | `/api/networks/:id/shares/:userId` | 取消分享 |
| POST | `/api/networks/:id/transfer` | 转移网络归属 |
| GET/POST | `/api/nodes` | 节点列表/创建（创建时自动签发凭据） |
| GET/PATCH/DELETE | `/api/nodes/:id` | 节点详情/更新/删除 |
| GET | `/api/nodes/:id/join` | 接入命令 / 配置 / 二维码数据 |
| POST | `/api/nodes/:id/rotate-credential` | 轮换节点凭据 |
| GET/POST | `/api/credentials` | 凭据列表/签发 |
| POST | `/api/credentials/:id/revoke` | 撤销凭据 |
| DELETE | `/api/credentials/:id` | 删除凭据记录 |
| GET | `/api/logs/audit` | 审计日志 |
| GET | `/api/logs/nodes` | 节点运行日志（分页检索） |
| GET | `/api/logs/stream` | 实时日志流（SSE） |
| DELETE | `/api/logs/nodes` | 清空节点日志 |
| GET | `/api/dashboard/{summary,traffic}` | 概览统计/流量曲线 |
| GET/PUT | `/api/system/settings` | EasyTier 运行参数与日志储存期限（管理员） |
| GET | `/api/system/{info,binaries}` | 系统信息/二进制检测 |

---

## 安全说明

- 管理员密码使用 **Argon2id**（64 MiB / 3 次迭代）哈希存储，并强制强密码策略。
- `network_secret`、`local_private_key`、凭据私钥等敏感字段使用 **AES-256-GCM** 加密后落库，密钥由 `APP_SECRET` 经 `scrypt` 派生。
- 登录态使用 httpOnly Cookie（SameSite=Lax），未配置 HTTPS 时请勿将 `COOKIE_SECURE` 设为误导性的 `true`，并在反向代理层启用 TLS。
- 建议启用 **安全模式（secure mode）**，共享节点固定 `local_private_key`，客户端可校验共享节点公钥，防止中间人。
- 临时凭据应遵循最小权限：默认不允许中继、不允许代理网段，并设置合理 TTL。

---

## 目录结构

```
.
├── backend/                 # Rust 后端（cargo crate）
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # 启动、优雅退出
│       ├── app.rs           # 路由装配、SPA/静态资源、CORS
│       ├── config.rs        # 环境变量与运行时设置
│       ├── db.rs            # rusqlite 连接与建表/查询
│       ├── error.rs         # 统一错误响应
│       ├── middleware.rs    # JWT 鉴权、限流、Cookie
│       ├── easytier/        # TOML 生成、CLI 封装、进程管理
│       ├── services/        # 网络/节点/凭据/日志/审计/指标
│       ├── routes/          # REST 路由 + SSE
│       └── util/            # 加解密、密码、X25519、格式化
├── frontend/                # Vue3 前端
│   └── src/{views,components,stores,api,router,layouts,utils}
├── Dockerfile
├── docker-compose.yml
└── .env.example
```

---

## 注意事项

- 控制台需能执行 `easytier-core` 且具备创建 TUN 设备的权限；容器内需 `NET_ADMIN`、`NET_RAW` 与 `/dev/net/tun`。
- 每个网络会占用一个监听端口与一个 RPC 端口，端口由控制台自动分配并持久化，可在「系统设置」中调整起始端口。
- 网络实例的日志文件写入 `DATA_DIR/logs/<网络ID>/`，控制台同时通过 SSE 实时推送并写入数据库。
- 若 `easytier-core` 版本较旧，部分字段（如凭据、secure mode）可能不可用，界面会给出相应提示。

---

## 接入地址（peer）选择

节点接入命令中的 `-p <peer>` 是客户端与网络会合的关键，需满足「客户端与共享节点都能到达同一地址」：

| 场景 | 推荐的 `-p` |
| --- | --- |
| 客户端与共享节点在同一局域网 | 共享节点宿主机的局域网地址 + 映射端口，如 `tcp://192.168.3.88:11011` |
| 共享节点有公网 IP 且已开放端口 | 共享节点公网地址 + 端口，如 `tcp://<公网IP>:11011` |
| 共享节点在 NAT 后且无公网入口 | 一个双方都能连上的公共共享节点/中继，如 `tcp://<中继>:11010`（须同时将该地址填到网络的「初始 Peer」，让共享节点也连上去） |

实现细节：

- 生成接入命令时，peer 取值优先级为：**手动指定 > 映射监听器（mapped_listeners）> 初始 Peer（external_node）> 初始 Peer 列表 > 全局默认**；
- 可在「网络编辑 → 映射监听器」中填写共享节点的对外可达地址（如 `tcp://192.168.3.88:11011`），接入命令会自动采用；
- 也可在节点「接入」弹窗中直接修改「连接地址 (peer)」后重新生成；
- 若使用公共中继，务必把它同时配置为网络的「初始 Peer」，否则共享节点不会连接到该中继，客户端也就找不到它。

> 注意：`public.easytier.cn` 等公共节点可能因网络/区域原因不可用；请以实际可达地址为准。

---

## 临时凭据的启动方式（重要）

EasyTier 2.x 中，**临时凭据必须通过命令行 `--credential` 传入**。仅靠配置文件里的 `[secure_mode] local_private_key` 不会进入凭据认证模式，连接时会报：

```
authentication failed: invalid proof and unknown credential
```

因此：

- 推荐直接使用控制台「接入」弹窗中的**一键运行命令**；
- 若想用配置文件，请用以下方式启动（即把凭据作为命令行参数）：

```bash
easytier-core -c config.toml --credential <凭据密钥>
```

控制台「配置文件」标签页已内置该提示与可复制的启动命令。

---

## 许可证

本项目仅用于管理 EasyTier，EasyTier 本身遵循其仓库许可证。
