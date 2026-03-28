.PHONY: install seed build dev backend frontend setup start prod

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
