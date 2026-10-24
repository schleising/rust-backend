FROM alpine:latest

RUN apk update && apk upgrade

RUN addgroup -S app && adduser -S -G app app

WORKDIR /app

COPY --chown=app:app secrets /app/secrets

COPY --chown=app:app dist/rust-backend /app/rust-backend

USER app

ENTRYPOINT [ "/app/rust-backend" ]
