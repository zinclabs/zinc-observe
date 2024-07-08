#!/usr/bin/sh
cd .. &&\
cargo build && \
cp target/debug/openobserve local-dev && \
cd local-dev && docker build -t github.com/ansrivas/o2local:latest -f Dockerfile .
