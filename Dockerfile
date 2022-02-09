FROM rust:1.58

WORKDIR /app
VOLUME /app
COPY . .

# Use exist Cargo.toml or initialize it.
RUN if [ ! -f "Cargo.toml" ]; then cargo init . ; fi
RUN cargo install --path .
