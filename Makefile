.PHONY: dev dev-local fresh prod install backup backup-test

VPS_IP       := 170.75.161.160
FRONTEND_URL := http://$(VPS_IP):3001
API_URL      := http://$(VPS_IP):5000

# ── Utility ────────────────────────────────────────────────────────────────────

install:
	cd apps/frontend && npm install

backup-test:
	cd apps/backend && cargo test --test test_backup

backup:
	@set -e; \
	DB_PATH=$$(grep '^DATABASE_URL=' apps/backend/.env | sed 's|^DATABASE_URL=sqlite:||'); \
	BACKUP_DIR="$(CURDIR)/sqlite/backups"; \
	mkdir -p "$$BACKUP_DIR"; \
	DEST="$$BACKUP_DIR/bsds-$$(date +%Y%m%d_%H%M%S).sqlite3"; \
	sqlite3 "$$DB_PATH" ".backup $$DEST"; \
	echo "==> Backed up to $$DEST"

# ── Production builds ──────────────────────────────────────────────────────────

# Seed data + test logins, no forced password reset
dev:
	@set -e; \
	printf 'NEXT_PUBLIC_API_URL=$(API_URL)\n' > apps/frontend/.env.local; \
	if grep -q '^FRONTEND_URL=' apps/backend/.env 2>/dev/null; then \
		sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=$(FRONTEND_URL)|' apps/backend/.env; \
	else \
		printf 'FRONTEND_URL=$(FRONTEND_URL)\n' >> apps/backend/.env; \
	fi; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	echo "==> Seeding database..."; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/seed); \
	echo "==> Building frontend..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001 -H 0.0.0.0) & \
	wait

# Same as dev but for local testing (localhost URLs, no 0.0.0.0 binding)
dev-local:
	@set -e; \
	printf 'NEXT_PUBLIC_API_URL=http://localhost:5000\n' > apps/frontend/.env.local; \
	if grep -q '^FRONTEND_URL=' apps/backend/.env 2>/dev/null; then \
		sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=http://localhost:3001|' apps/backend/.env; \
	else \
		printf 'FRONTEND_URL=http://localhost:3001\n' >> apps/backend/.env; \
	fi; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	echo "==> Seeding database..."; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/seed); \
	echo "==> Building frontend..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001) & \
	wait

# Clean slate — generates a new .env on first run, then builds + bootstraps on second run
fresh:
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
		printf 'FRONTEND_URL=$(FRONTEND_URL)\n\n' >> "$$ENV_FILE"; \
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
		printf 'NEXT_PUBLIC_API_URL=$(API_URL)\n' > apps/frontend/.env.local; \
		echo ""; \
		echo "==> $$ENV_FILE generated (DB: $$DB_DIR)."; \
		echo ""; \
		echo "    Edit staff entries (emails, names, phones, passwords), then re-run:"; \
		echo "      make fresh"; \
		echo ""; \
		exit 0; \
	fi; \
	printf 'NEXT_PUBLIC_API_URL=$(API_URL)\n' > apps/frontend/.env.local; \
	sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=$(FRONTEND_URL)|' "$$ENV_FILE"; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	echo "==> Bootstrapping staff accounts..."; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bootstrap); \
	echo "==> Building frontend..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001 -H 0.0.0.0) & \
	wait

# Real production — no seed data, bootstrap staff with forced password reset
prod:
	@set -e; \
	printf 'NEXT_PUBLIC_API_URL=$(API_URL)\n' > apps/frontend/.env.local; \
	if grep -q '^FRONTEND_URL=' apps/backend/.env 2>/dev/null; then \
		sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=$(FRONTEND_URL)|' apps/backend/.env; \
	else \
		printf 'FRONTEND_URL=$(FRONTEND_URL)\n' >> apps/backend/.env; \
	fi; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	echo "==> Bootstrapping staff accounts..."; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bootstrap); \
	echo "==> Building frontend..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001 -H 0.0.0.0) & \
	wait
