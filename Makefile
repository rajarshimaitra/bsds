.PHONY: dev dev-local fresh prod install backup backup-test

VPS_IP       := 170.75.161.160
FRONTEND_URL := http://$(VPS_IP):3001
API_URL      := http://$(VPS_IP):5000
STAFF_TOML   := $(CURDIR)/staff.toml

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

# ── Setup ──────────────────────────────────────────────────────────────────────

# Generate env/configs only. No compilation, no seed, no start.
# Run once, edit staff.toml, then use make prod or make dev.
fresh:
	@set -e; \
	NEEDS_INPUT=0; \
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
		printf 'WHATSAPP_API_TOKEN=\n' >> "$$ENV_FILE"; \
		echo "==> $$ENV_FILE generated (DB: $$DB_DIR)."; \
		NEEDS_INPUT=1; \
	fi; \
	if [ ! -f "$(STAFF_TOML)" ]; then \
		cp staff.toml.example "$(STAFF_TOML)"; \
		echo "==> staff.toml created from staff.toml.example."; \
		NEEDS_INPUT=1; \
	fi; \
	if [ "$$NEEDS_INPUT" = "1" ]; then \
		echo ""; \
		echo "    Edit staff.toml with real names, emails, phones, and passwords,"; \
		echo "    then run: make prod  or  make dev"; \
		echo ""; \
		exit 0; \
	fi; \
	sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=$(FRONTEND_URL)|' "$$ENV_FILE"; \
	echo "==> Config ready. Run 'make prod' or 'make dev'."

# ── Start targets (compile + run) ──────────────────────────────────────────────

# Compile + bootstrap staff from staff.toml (forced password reset) + start.
prod:
	@set -e; \
	printf 'NEXT_PUBLIC_API_URL=$(API_URL)\n' > apps/frontend/.env.local; \
	sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=$(FRONTEND_URL)|' apps/backend/.env; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	echo "==> Bootstrapping staff accounts from staff.toml..."; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bootstrap --config "$(STAFF_TOML)"); \
	echo "==> Installing frontend dependencies..."; \
	(cd apps/frontend && npm install); \
	echo "==> Building frontend..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001 -H 0.0.0.0) & \
	wait

# Compile + seed test data + start. Test logins enabled, no forced password reset.
dev:
	@set -e; \
	printf 'NEXT_PUBLIC_API_URL=$(API_URL)\nNEXT_PUBLIC_TEST_MODE=true\n' > apps/frontend/.env.local; \
	sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=$(FRONTEND_URL)|' apps/backend/.env; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	DB_PATH=$$(grep '^DATABASE_URL=' apps/backend/.env | sed 's|^DATABASE_URL=sqlite:||'); \
	echo "==> Seeding database: $$DB_PATH"; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/seed); \
	echo "==> Installing frontend dependencies..."; \
	(cd apps/frontend && npm install); \
	echo "==> Building frontend (test mode)..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001 -H 0.0.0.0) & \
	wait

# Same as dev but bound to localhost only.
dev-local:
	@set -e; \
	printf 'NEXT_PUBLIC_API_URL=http://localhost:5000\nNEXT_PUBLIC_TEST_MODE=true\n' > apps/frontend/.env.local; \
	sed -i 's|^FRONTEND_URL=.*|FRONTEND_URL=http://localhost:3001|' apps/backend/.env; \
	echo "==> Building backend..."; \
	(cd apps/backend && cargo build --release); \
	DB_PATH=$$(grep '^DATABASE_URL=' apps/backend/.env | sed 's|^DATABASE_URL=sqlite:||'); \
	echo "==> Seeding database: $$DB_PATH"; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/seed); \
	echo "==> Installing frontend dependencies..."; \
	(cd apps/frontend && npm install); \
	echo "==> Building frontend (test mode)..."; \
	(cd apps/frontend && npm run build); \
	echo "==> Starting — Ctrl+C stops both."; \
	trap 'kill 0' INT; \
	(cd apps/backend && set -a && . ./.env && set +a && ./target/release/bsds-backend) & \
	(cd apps/frontend && npm start -- -p 3001) & \
	wait
