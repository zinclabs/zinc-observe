# syntax=docker/dockerfile:1
FROM public.ecr.aws/docker/library/node:18.20.4-bookworm AS webBuilder
WORKDIR /web
COPY ./web /web/

RUN npm install
RUN NODE_OPTIONS="--max-old-space-size=8192" npm run build

# FROM public.ecr.aws/docker/library/rust:bookworm as builder
FROM public.ecr.aws/zinclabs/rust:bookworm-sccache AS builder

# RUN rustup toolchain install nightly-2024-07-07
# RUN rustup default nightly-2024-07-07
# RUN rustup target add x86_64-unknown-linux-gnu
# RUN diff -u <(rustc --print cfg) <(rustc -C target-cpu=native --print cfg)
RUN rustc --version && sccache --version

WORKDIR /openobserve
COPY . /openobserve
COPY --from=webBuilder /web/dist web/dist

RUN cargo build --release

# FROM gcr.io/distroless/cc as runtime
FROM public.ecr.aws/debian/debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates
RUN apt-get install -y curl htop iftop sysstat procps lsof net-tools
RUN update-ca-certificates
COPY --from=builder /openobserve/target/release/openobserve /
RUN ["/openobserve", "init-dir", "-p", "/data/"]
CMD ["/openobserve"]
