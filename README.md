# bsds-dashboard

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 18+
- npm
- `openssl` (for secret generation in `make prod-fresh`)

## Development

First time only:

```bash
make setup
```

This installs frontend dependencies and seeds the database with test accounts.

```bash
make dev
```

Starts the backend at **http://localhost:5000** and the frontend at **http://localhost:3000**. Press `Ctrl+C` to stop both.

## Production (fresh deploy)

```bash
make prod-fresh
```

**First run** — generates `apps/backend/.env` with fresh secrets, a timestamped database path, and placeholder staff entries, then exits with instructions.

**Edit the staff entries** before proceeding:

```bash
nano apps/backend/.env
```

Set real values for:

```env
BOOTSTRAP_ADMIN_EMAIL=admin@yourdomain.com
BOOTSTRAP_ADMIN_NAME=Full Name
BOOTSTRAP_ADMIN_PHONE=9800000000
BOOTSTRAP_ADMIN_PASSWORD=StrongPassword@1

BOOTSTRAP_OPERATOR_EMAIL=...
BOOTSTRAP_ORGANISER_EMAIL=...
```

**Second run** — builds both services, creates the database, bootstraps staff accounts (existing accounts are skipped), and starts everything:

```bash
make prod-fresh
```

Backend runs on **http://localhost:5000**, frontend on **http://localhost:3000**.

All bootstrap accounts are created with `is_temp_password=true` — users are forced to change their password on first login.

### Production database

The database is stored locally under `./sqlite/prod-<YYYYMMDD_HHMMSS>/bsds.sqlite3`, timestamped at creation. It is excluded from git. To start over with a fresh database, delete `apps/backend/.env` and re-run `make prod-fresh`.

### Re-deploying / restarting

`apps/backend/.env` persists between runs. To restart without rebuilding:

```bash
make start
```

To rebuild and restart with the existing database and config:

```bash
make prod
```

## Commands

| Command | Description |
|---|---|
| `make prod-fresh` | Full fresh production deploy (generates config, builds, bootstraps, starts) |
| `make prod` | Build and run in production mode (existing `.env` and DB) |
| `make start` | Run production binaries without rebuilding |
| `make build` | Production build only (backend + frontend) |
| `make dev` | Run both services in development mode |
| `make backend` | Run backend only (dev) |
| `make frontend` | Run frontend only (dev) |
| `make seed` | Re-seed the database with test data (dev only) |
