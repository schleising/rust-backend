# rust-backend

## Cross Compilation

In order to cross compile the backend for an Alpine based Docker image, you need to install the following packages:

```bash
# Install the target and the toolchain
rustup target add aarch64-unknown-linux-musl 
rustup target add --toolchain stable-aarch64-unknown-linux-musl aarch64-unknown-linux-musl

# Install the musl toolchain from homebrew
brew tap messense/macos-cross-toolchains
brew install aarch64-unknown-linux-gnu
```

You will also need to edit (or create) the `~/.cargo/config.toml` file and add the following lines:

```toml
[target.aarch64-unknown-linux-musl]
linker = "aarch64-linux-musl-gcc"
```

This will tell Rust to use the `aarch64-linux-musl-gcc` linker when building for the `aarch64-unknown-linux-musl` target.

Then you can build the backend for the target architecture using the following command:

```bash
# Build the backend for the target architecture
cargo build --target aarch64-unknown-linux-musl --release
```

## Makefile
The makefile will build the backend for various deployments

 - release [Default]
    - This builds the backend and copies it into a container
    - The container is then run with the backend
    - The backend is built for the target architecture
    - This build is for a server already running a MongoDB instance
 - dev
    - This builds the backend and copies it into a container and also downloads a MongoDB image
    - Both the backend and MongoDB are run in separate containers
 - local
    - The backend is built for the local architecture and runs locally
    - The MongoDB instance is run in a container
    - This build is for a local development environment
 - stop
    - This stops the containers and removes them
    - This will remove all the containers created by the makefile
 - clean
    - This cleans up the build artifacts and the containers
    - This will remove all the containers and images created by the makefile
