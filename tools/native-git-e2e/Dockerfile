FROM rust:bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        git \
        gnupg \
        openssh-client \
        openssh-server \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

COPY . /workspace

RUN cargo build

COPY tools/native-git-e2e/entrypoint.sh /usr/local/bin/committy-native-git-e2e
RUN chmod +x /usr/local/bin/committy-native-git-e2e

ENTRYPOINT ["/usr/local/bin/committy-native-git-e2e"]
