FROM rust:alpine AS build
WORKDIR /build/xkcd-bot

RUN apk add build-base pkgconf curl-dev openssl-dev
ENV RUSTFLAGS=-Ctarget-feature=-crt-static

COPY . .
RUN cargo install --path ./bot --root /

FROM alpine

RUN apk add gcc curl

COPY --from=build /bin/xkcd-bot /xkcd-bot
ENTRYPOINT /xkcd-bot
