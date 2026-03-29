# bsds-dashboard

## VPS Setup

Before deploying, install the following system packages on your server (Debian/Ubuntu):

```bash
apt update && apt install -y \
  git \
  curl \
  build-essential \
  pkg-config \
  libssl-dev \
  openssl \
  sqlite3 \
  net-tools \
  cron \
  bpytop \
  nginx \
  certbot \
  python3-certbot-nginx
```

Install **Rust** (stable toolchain):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Install **Node.js** 18+ via NodeSource:

```bash
curl -fsSL https://deb.nodesource.com/setup_18.x | bash -
apt install -y nodejs
```

Ensure `cron` is running:

```bash
systemctl enable cron && systemctl start cron
```

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

## Automated database backups

Backups use `sqlite3`'s online backup API (safe during live writes) and are stored under `./sqlite/backups/`. All backup files are retained indefinitely.

**Prerequisite:** the `sqlite3` CLI must be installed on the server.

```bash
apt install sqlite3   # Debian/Ubuntu
```

**Run a backup manually:**

```bash
make backup
```

**Schedule daily backups via cron** — SSH into the server and run `crontab -e`, then add:

```
# 2:00am — take backup
0 2 * * * cd /path/to/bsds-dashboard && make backup >> /var/log/bsds-backup.log 2>&1

# 2:10am — verify backup mechanism still works
10 2 * * * cd /path/to/bsds-dashboard && make backup-test >> /var/log/bsds-backup.log 2>&1
```

Replace `/path/to/bsds-dashboard` with the absolute path to this repo on the server.

The `backup-test` target runs three integration tests that confirm the backup file was created, all tables are present, and the backup is an independent snapshot of the data at backup time. Failures are logged to `/var/log/bsds-backup.log` with a non-zero exit code, which you can pipe to an alert:

```
10 2 * * * cd /path/to/bsds-dashboard && make backup-test >> /var/log/bsds-backup.log 2>&1 || echo "backup-test failed" | mail -s "BSDS backup alert" you@example.com
```

## Connecting to a domain

The app runs two services: the **frontend** on port 3000 and the **backend** on port 5000. To serve them publicly under a domain you need a reverse proxy (Nginx or Caddy) and updated environment variables.

### 1. Point DNS to your server

In your domain registrar or DNS provider, create an **A record** pointing your domain to your server's public IP:

```
Type  Name   Value
A     @      <your-server-ip>       # yourdomain.com → frontend
A     api    <your-server-ip>       # api.yourdomain.com → backend
```

Use `api.yourdomain.com` as a subdomain for the backend, or choose any subdomain you prefer.

### 2. Update environment variables

Edit `apps/backend/.env` and change the two URL fields to your real domain:

```env
FRONTEND_URL=https://yourdomain.com
```

Edit (or create) `apps/frontend/.env.production.local` and set:

```env
NEXT_PUBLIC_API_URL=https://api.yourdomain.com
```

Then rebuild and restart:

```bash
make prod
```

### 3. Set up Nginx reverse proxy

Install Nginx if needed:

```bash
apt install nginx
```

Create `/etc/nginx/sites-available/bsds`:

```nginx
# Frontend
server {
    listen 80;
    server_name yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}

# Backend API
server {
    listen 80;
    server_name api.yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:5000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Enable the config and reload:

```bash
ln -s /etc/nginx/sites-available/bsds /etc/nginx/sites-enabled/
nginx -t && systemctl reload nginx
```

### 4. Enable HTTPS with Let's Encrypt

```bash
apt install certbot python3-certbot-nginx
certbot --nginx -d yourdomain.com -d api.yourdomain.com
```

Certbot will automatically update your Nginx config to use HTTPS and redirect HTTP. After this, make sure `apps/backend/.env` has `FRONTEND_URL=https://yourdomain.com` and `apps/frontend/.env.production.local` has `NEXT_PUBLIC_API_URL=https://api.yourdomain.com`, then run `make prod` once more to apply the changes.

---

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
| `make backup` | Back up the production database to `./sqlite/backups/` |
| `make backup-test` | Run backup integration tests (requires `sqlite3` CLI) |
