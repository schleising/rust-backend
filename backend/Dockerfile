# Create a new stage for the final image
FROM alpine:latest

# Update and upgrade the image
RUN apk update && apk upgrade

# Set the Current Working Directory inside the container
WORKDIR /app

# Copy the secrets directory
COPY secrets /app/secrets

# Copy the compiled binary from the builder stage
COPY target/aarch64-unknown-linux-musl/release/rust-backend /app/rust-backend

# Command to run the executable
ENTRYPOINT [ "/app/rust-backend" ]
