# proxy-rs
A minimal, high-performance TCP proxy written in [Rust](https://rust-lang.org/) using [Tokio](https://tokio.rs/).

Designed for low latency and fast connection handling, with a warm connection mechanism to reduce connection setup delays. Suitable for game servers (such as Minecraft) or any raw TCP service.

---

## Features
* Raw TCP proxy (Layer 4, not HTTP)
* Asynchronous, non-blocking architecture powered by Tokio
* Warm connection pool to reduce latency on new connections
* Automatic fallback if warm connection is unavailable
* Optimized socket settings (`TCP_NODELAY`, `TCP_QUICKACK`)
* Timestamped connection logging
* Lightweight and minimal dependencies
* Docker support included

---

## How it works
The proxy listens on a local port and forwards all traffic to a configured remote host and port.

It maintains one pre-established ("warm") connection to the remote server so new clients can be connected instantly without waiting for TCP handshake overhead.

Traffic is forwarded bidirectionally until either side closes the connection.

---

## Configuration
Configuration is done via environment variables.

| Variable    | Default   | Description           |
| ----------- | --------- | --------------------- |
| LOCAL_PORT  | 25565     | Port to listen on     |
| REMOTE_HOST | 127.0.0.1 | Target server address |
| REMOTE_PORT | 25565     | Target server port    |

Example `.env`:

```
LOCAL_PORT=25565
REMOTE_HOST=example.com
REMOTE_PORT=25565
```

---

## Running locally
Requirements:

* Rust
* Cargo

Run:

```
cargo run --release
```

---

## Running with Docker
Build and run (and detach):

```
docker compose up -d --build
```

Or manually:

```
docker build -t proxy-rs .
docker run -p 25565:25565 --env-file .env proxy-rs
```

---

## Example use cases
* Minecraft TCP proxy
* Game server forwarding
* Port forwarding between hosts
* Reducing connection latency via warm pooling
* Simple TCP relay

---

## Project structure
```
.
├── proxy.rs
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── docker-compose.yml
├── template.docker-compose.yml
├── .env
├── template.env
```

---

## Notes
This proxy operates at the TCP level and does not inspect or modify traffic.

It works with any TCP protocol.

---

## License
[MIT](https://opensource.org/license/mit)
