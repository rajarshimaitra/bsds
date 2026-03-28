# bsds-dashboard

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 18+
- npm

## Setup

First time only:

```bash
make setup
```

This installs frontend dependencies and seeds the database.

## Run

```bash
make dev
```

Starts the backend at **http://localhost:5000** and the frontend at **http://localhost:3000**. Press `Ctrl+C` to stop both.

## Production

```bash
make prod
```

Builds and starts both services in production mode. To skip rebuilding:

```bash
make start
```

## Other commands

| Command | Description |
|---|---|
| `make build` | Production build (backend + frontend) |
| `make prod` | Build and run in production mode |
| `make start` | Run production binaries (requires prior `make build`) |
| `make seed` | Re-seed the database |
| `make backend` | Run backend only (dev) |
| `make frontend` | Run frontend only (dev) |
