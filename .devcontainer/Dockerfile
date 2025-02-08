# Use the official Rust image as a base
FROM rust:latest

# Install dependencies first (as root)
RUN apt update && apt install -y \
    zsh \
    bash-completion \
    curl \
    git \
    nano \
    vim \
    lsb-release \
    locales \
    && rm -rf /var/lib/apt/lists/*

# Set up locale for Unicode support
RUN locale-gen en_US.UTF-8
ENV LANG=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8

# Create a non-root user with Zsh as default shell
RUN useradd -m -s /usr/bin/zsh rust

# Set working directory
WORKDIR /workspace

# Set permissions
RUN chown -R rust:rust /workspace

# Switch to the new user
USER rust

# Install Rust components including rust-analyzer
RUN rustup component add rust-analyzer

# Install Oh My Zsh for a better shell experience
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" || true

# Default command when the container starts
CMD ["/usr/bin/zsh"]
