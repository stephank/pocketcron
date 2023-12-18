FROM rust AS build
WORKDIR /usr/src/pocketcron
COPY . .
RUN cargo install --path .

FROM debian
COPY --from=build \
  /usr/local/cargo/bin/pocketcron \
  /usr/bin/pocketcron
CMD ["pocketcron"]
