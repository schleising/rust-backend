# Create a new stage for the final image
FROM alpine:latest

# Update and upgrade the image
RUN apk update && apk upgrade

# Create a non-root user and group
RUN addgroup -S app && adduser -S -G app app

# Set the Current Working Directory inside the container
WORKDIR /app

# Copy the secrets directory
COPY --chown=app:app secrets /app/secrets

# Copy the compiled binary from the builder stage
COPY --chown=app:app target/aarch64-unknown-linux-musl/release/rust-backend /app/rust-backend

# Run as non-root
USER app

# Command to run the executable
ENTRYPOINT [ "/app/rust-backend" ]
