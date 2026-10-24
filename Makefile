release:
	docker compose -f docker-compose.yaml --profile build run --rm backend-builder
	docker compose -f docker-compose.yaml up --build -d

dev:
	docker compose -f docker-compose-dev.yaml --profile build run --rm backend-builder
	docker compose -f docker-compose-dev.yaml up --build -d

local:
	cd backend; cargo update; cargo build --release
	docker compose -f docker-compose-local.yaml up --build -d
	cd backend; target/release/rust-backend

stop:
	docker compose -f docker-compose.yaml down
	docker compose -f docker-compose-dev.yaml down
	docker compose -f docker-compose-local.yaml down

clean:
	cd backend; cargo clean
