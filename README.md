# rust-backend

## Builds

`release` and `dev` compile the backend in a `rust:1-alpine` container, so the binary is Linux musl for whatever architecture the Docker daemon runs. On Apple Silicon that is `aarch64-unknown-linux-musl`. No host cross-compilation toolchain is required.

Every build runs `cargo generate-lockfile` and `cargo update`, then copies `backend/dist/rust-backend` into the Alpine image. The Cargo registry and `target` directory are named volumes (`rust-backend-cargo-registry`, `rust-backend-cargo-git`, and `rust-backend-cargo-target`). Unchanged crates are not recompiled, and `docker system prune` leaves those volumes in place. `docker volume prune -a` and `docker compose down -v` remove them.

The `local` target still compiles on the host with the default Rust target and runs that binary directly. It needs a host Rust toolchain, and it does not use the image builder.

Host musl linkers and `~/.cargo/config.toml` target linker settings are unused by this project.

## Makefile
The makefile will build the backend for various deployments

 - release [Default]
    - This compiles the backend inside Docker and starts the container
    - Dependencies are upgraded on every build
    - Unchanged crates are reused from named Docker volumes
    - This build is for a server already running a MongoDB instance
 - dev
    - This compiles the backend inside Docker and also downloads a MongoDB image
    - Both the backend and MongoDB are run in separate containers
    - Dependencies are upgraded on every build
 - local
    - The backend is built for the host architecture and runs locally
    - The MongoDB instance is run in a container
    - This build is for a local development environment
 - stop
    - This stops the containers and removes them
    - This will remove all the containers created by the makefile
 - clean
    - This removes host build artifacts under `backend/target`
    - The named Cargo volumes are kept, so the next Docker build stays incremental
