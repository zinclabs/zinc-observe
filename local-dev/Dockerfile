# syntax=docker/dockerfile:1

FROM fedora:40 AS runtime

RUN dnf update -y && \
    dnf install -y \
    atk \
    nss \
    glib2 \
    ca-certificates \
    curl \
    htop \
    iftop \
    sysstat \
    procps-ng \
    lsof \
    net-tools \
    sqlite && \
    dnf clean all && \
    update-ca-trust

COPY ./openobserve /

RUN ["/openobserve", "init-dir", "-p", "/data/"]

CMD ["/openobserve"]
