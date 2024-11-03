# Zero To Production In Rust

Email newsletter service using Axum

## Environment Setup

### pre-commit

pre-commit is a code checking tool that can perform code checks before committing the code.

```bash
pip install pre-commit
```

After successful installation, you can run `pre-commit install` to set it up.

### Cargo deny

Cargo deny is a Cargo plugin that can be used to check the security of dependencies.

```bash
cargo install --locked cargo-deny
```

### typos

typos is a spell checking tool.

```bash
cargo install typos-cli
```

### git cliff

git cliff is a tool for generating changelogs.

```bash
cargo install git-cliff
```

### cargo nextest

cargo nextest is an enhanced testing tool for Rust.

```bash
cargo install cargo-nextest --locked
```

### Nix

You can install Nix to manage dependent services such as PostgreSQL.
[Go to install.](https://nixos.org/download/)

If you have already installed, execute `nix-shell` to start the dependent services.
