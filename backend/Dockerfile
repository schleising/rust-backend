# Start with the golang image
FROM rust:alpine AS builder

# Update and upgrade the image
RUN apk update && apk upgrade
RUN apk add --no-cache musl-dev

# Set the Current Working Directory inside the container
WORKDIR /app

# Copy source and secret files
COPY Cargo.toml /app/
COPY src/ /app/src/
COPY secrets/ /app/secrets/

# Build and install the app
RUN cargo install --path .

# Command to run the executable
ENTRYPOINT [ "rust-backend" ]
