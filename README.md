# Committy

🚀 Generate clear, concise, and structured commit messages effortlessly with Committy!

## 🌟 Features

- Generate commit messages compatible with SemVer specification
- Support for various commit types (feat, fix, build, etc.)
- Option to indicate breaking changes
- Short and long commit message support
- Amend existing commits
- Easy-to-use CLI interface

## 🤔 Why Committy?

Committy was born out of the need for a simple, efficient tool to generate structured commit messages. Whether you're working on a personal project or collaborating with a team, Committy helps you:

- Maintain a clean and consistent commit history
- Easily generate changelogs
- Adhere to commit message best practices
- Save time on writing detailed commit messages

Plus, it's a great way to learn and practice Rust programming!

## 🚀 Quick Start

### Installation

```
curl -fsSL https://raw.githubusercontent.com/martient/committy/refs/heads/main/install.sh | bash
```

### Basic Usage

To generate a commit message:

```shell
committy
```

#### Demo

![Commit demo](docs/public/demos/commit.gif)

## 📚 Documentation

- Full docs live in `docs/` (Astro + Starlight): `docs/src/content/docs/`
- Key references:
  - Group Commit: `docs/src/content/docs/reference/group-commit.mdx`
  - AI Flags & Security: `docs/src/content/docs/reference/ai-flags.mdx`

## 🛠 Options and Commands

### Amend an existing commit

```shell
committy amend
```

#### Demo

![Amend demo](docs/public/demos/amend.gif)

### Create a short commit

```shell
committy -s "change the api version"
```

### Create a short commit and amend

```shell
committy -s "change the api version" amend
```

## ⚙️ CLI Reference & Advanced Usage

### Output format

- Use `--output json|text` on commands that support it.
- `lint --output json` prints `{ ok, count, issues }`.
- `tag --output json` (with `--dry-run`) prints `{ ok, new_tag }`.

### Verbosity

- `-v` or `--verbose`: increase log verbosity. Repeat for more details (`-vv`).
- `-q` or `--quiet`: only error logs.
- Defaults to `info` when neither is provided.

### Non-interactive mode

- `--non-interactive` disables prompts (ideal for CI).
- Also enabled when `COMMITTY_NONINTERACTIVE=1` or `CI=1`.

### Fetch control for tags

- `--fetch` / `--no-fetch` controls whether tags are fetched from remote before calculation.
- Default: fetch is enabled unless `--no-fetch` is provided.
- Example (no remote access):

```bash
committy --non-interactive tag --no-fetch --dry-run --not-publish --output json
```

### Stable lint exit codes (for CI)

- `0` = OK, no issues
- `3` = Lint issues found
- `1` = Error

Example:

```bash
committy --non-interactive lint --repo-path . --output json || {
  code=$?; if [ $code -eq 3 ]; then echo "Lint issues"; else exit $code; fi
}
```

### Configurable version bump rules

Committy determines semantic version bumps using regex patterns loaded from `config.toml`.

- Default location: `~/.config/committy/config.toml`
- Override directory with `COMMITTY_CONFIG_DIR` (the file must be named `config.toml`).
- Keys:
  - `major_regex`
  - `minor_regex`
  - `patch_regex`

Example `config.toml` (use single quotes for literal regex):

```toml
minor_regex = '(?im)^fix(?:\s*\([^)]*\))?:'
patch_regex = '(?im)^docs(?:\s*\([^)]*\))?:'
major_regex = '(?im)^(breaking change:|feat(?:\s*\([^)]*\))?!:)'
```

This example treats `fix:` commits as a minor bump (instead of patch) and moves `docs:` to patch.

## 📝 Commit Types

Committy supports the following commit types:

- feat: New feature
- fix: Bug fix
- build: Changes that affect the build system or external dependencies
- chore: Other changes that don't modify src or test files
- ci: Changes to CI configuration files and scripts
- cd: Changes to CD configuration files and scripts
- docs: Documentation only changes
- perf: A code change that improves performance
- refactor: A code change that neither fixes a bug nor adds a feature
- revert: Revert a previous commit
- style: Changes that do not affect the meaning of the code
- test: Adding missing tests or correcting existing tests

## 💥 Breaking Changes

When prompted, you can indicate if your commit includes breaking changes. This will add a `!` at the end of the commit type, signaling a breaking change as per conventional commit standards.

## 📄 Commit Message Structure

### Short Commit Message

A brief, concise summary of the changes (around 150 characters). Think of it as a TL;DR for your commit.

### Long Commit Message

A more detailed explanation of the changes, which will be included in the changelog. Use this to provide context, reasoning, and any other relevant information about your changes.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create a new branch for your feature or bug fix
3. Make your changes and commit them
4. Push your changes to your forked repository
5. Create a pull request to the 'develop' branch of the main repository

Please see the [CONTRIBUTING](CONTRIBUTING) file for more details.

## 📜 License

This project is licensed under the Apache 2.0 License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by the need for consistent commit messages
- Built with love using Rust 🦀