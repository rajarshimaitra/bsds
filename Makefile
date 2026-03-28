.PHONY: install seed build dev backend frontend setup start prod prod-fresh

# First-time setup: install JS deps and seed the database
setup: install seed

# Install frontend npm dependencies
install:
	cd apps/frontend && npm install

# Seed the database (run once, or to reset data)
seed:
	cd apps/backend && cargo run --bin seed

# Build both services for production
build:
	cd apps/backend && cargo build --release
	cd apps/frontend && npm run build

# Run both services concurrently (Ctrl+C stops both)
dev:
	@trap 'kill 0' INT; \
	  (cd apps/backend && cargo run --bin bsds-backend) & \
	  (cd apps/frontend && npm run dev -- -p 3000) & \
	  wait

# Run backend only
backend:
	cd apps/backend && cargo run --bin bsds-backend

# Run frontend only
frontend:
	cd apps/frontend && npm run dev -- -p 3000

# Run both services in production mode (must run `make build` first)
start:
	@trap 'kill 0' INT; \
	  (cd apps/backend && ./target/release/bsds-backend) & \
	  (cd apps/frontend && npm start -- -p 3000) & \
	  wait

# Build and run both services in production mode
prod: build start

# Step 1 — run once to generate apps/backend/.env, then edit staff entries.
# Step 2 — run again to build, bootstrap, and start.
prod-fresh:
	@set -e; \
	ENV_FILE="apps/backend/.env"; \
	if [ ! -f "$$ENV_FILE" ]; then \
		TIMESTAMP=$$(date +%Y%m%d_%H%M%S); \
		DB_DIR="$(CURDIR)/sqlite/prod-$$TIMESTAMP"; \
		SESSION_SECRET=$$(openssl rand -base64 32 | tr -d '\n='); \
		ENCRYPTION_KEY=$$(openssl rand -hex 32); \
		mkdir -p "$$DB_DIR"; \
		printf '# ── Database ──────────────────────────────────────────────────────────────────\n' > "$$ENV_FILE"; \
		printf 'DATABASE_URL=sqlite:%s/bsds.sqlite3\n\n' "$$DB_DIR" >> "$$ENV_FILE"; \
		printf '# ── Security (auto-generated — do not share or commit) ────────────────────────\n' >> "$$ENV_FILE"; \
		printf 'SESSION_SECRET=%s\n' "$$SESSION_SECRET" >> "$$ENV_FILE"; \
		printf 'ENCRYPTION_KEY=%s\n' "$$ENCRYPTION_KEY" >> "$$ENV_FILE"; \
		printf 'FRONTEND_URL=http://localhost:3000\n\n' >> "$$ENV_FILE"; \
		printf '# ── Integrations (optional) ───────────────────────────────────────────────────\n' >> "$$ENV_FILE"; \
		printf 'RAZORPAY_KEY_ID=\n' >> "$$ENV_FILE"; \
		printf 'RAZORPAY_KEY_SECRET=\n' >> "$$ENV_FILE"; \
		printf 'RAZORPAY_WEBHOOK_SECRET=\n' >> "$$ENV_FILE"; \
		printf 'WHATSAPP_API_URL=\n' >> "$$ENV_FILE"; \
		printf 'WHATSAPP_API_TOKEN=\n\n' >> "$$ENV_FILE"; \
		printf '# ── Bootstrap staff accounts (temp passwords — forced change on first login) ───\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ADMIN_EMAIL=admin@yourdomain.com\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ADMIN_NAME=Admin\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ADMIN_PHONE=9800000000\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ADMIN_PASSWORD=Admin@123\n\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_OPERATOR_EMAIL=operator@yourdomain.com\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_OPERATOR_NAME=Operator\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_OPERATOR_PHONE=9800000001\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_OPERATOR_PASSWORD=Operator@123\n\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ORGANISER_EMAIL=organiser@yourdomain.com\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ORGANISER_NAME=Organiser\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ORGANISER_PHONE=9800000002\n' >> "$$ENV_FILE"; \
		printf 'BOOTSTRAP_ORGANISER_PASSWORD=Organiser@123\n' >> "$$ENV_FILE"; \
		printf 'NEXT_PUBLIC_API_URL=http://localhost:5000\n' > apps/frontend/.env.production.local; \
		echo ""; \
		echo "==> $$ENV_FILE generated."; \
		echo ""; \
		echo "    Edit staff entries (emails, names, phones, passwords), then re-run:"; \
		echo "      make prod-fresh"; \
		echo ""; \
		exit 0; \
	fi; \
	echo "==> Building backend (release)..."; \
	(cd apps/backend && cargo build --release); \
	echo "==> Building frontend (production)..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Bootstrapping staff accounts..."; \
	(cd apps/backend && cargo run --bin bootstrap); \
	echo ""; \
	echo "==> Starting — Ctrl+C stops both services."; \
	trap 'kill 0' INT; \
	(cd apps/backend && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3000) & \
	wait
