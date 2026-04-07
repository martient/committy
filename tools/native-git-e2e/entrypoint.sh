#!/usr/bin/env bash
set -euo pipefail

export HOME=/tmp/committy-e2e-home
export GNUPGHOME="$HOME/.gnupg"
export COMMITTY_NONINTERACTIVE=1
export CI=1

committy_bin="/workspace/target/debug/committy"

log() {
  printf '[native-git-e2e] %s\n' "$1"
}

run_git() {
  local repo_path="$1"
  shift
  git -C "$repo_path" "$@"
}

ensure_signing_config() {
  mkdir -p "$GNUPGHOME" "$HOME/.ssh"
  chmod 700 "$GNUPGHOME" "$HOME/.ssh"

  cat > /tmp/committy-e2e-gpg-batch <<'EOF'
%no-protection
Key-Type: RSA
Key-Length: 3072
Subkey-Type: RSA
Subkey-Length: 3072
Name-Real: Committy E2E
Name-Email: committy-e2e@example.com
Expire-Date: 0
EOF

  gpg --batch --generate-key /tmp/committy-e2e-gpg-batch

  local fingerprint
  fingerprint="$(gpg --batch --list-secret-keys --with-colons committy-e2e@example.com | awk -F: '/^fpr:/ { print $10; exit }')"
  if [[ -z "$fingerprint" ]]; then
    echo "failed to resolve generated signing key fingerprint" >&2
    exit 1
  fi

  printf '%s:6:\n' "$fingerprint" | gpg --import-ownertrust

  git config --global user.name "Committy E2E"
  git config --global user.email "committy-e2e@example.com"
  git config --global user.signingkey "$fingerprint"
  git config --global commit.gpgsign true
  git config --global tag.gpgsign true
  git config --global gpg.program gpg
  git config --global init.defaultBranch main

  ssh-keygen -t ed25519 -N '' -C committy-e2e@example.com -f "$HOME/.ssh/signing_key" >/dev/null
  printf 'committy-e2e@example.com %s\n' "$(cat "$HOME/.ssh/signing_key.pub")" > "$HOME/.ssh/allowed_signers"
  chmod 600 "$HOME/.ssh/allowed_signers"
}

create_remote_user() {
  local user_name="$1"
  local key_file="$2"

  useradd --create-home --shell /usr/bin/git-shell "$user_name"
  mkdir -p "/home/$user_name/.ssh" "/home/$user_name/repos"
  chmod 700 "/home/$user_name/.ssh"
  cat "${key_file}.pub" > "/home/$user_name/.ssh/authorized_keys"
  chmod 600 "/home/$user_name/.ssh/authorized_keys"
  chown -R "$user_name:$user_name" "/home/$user_name/.ssh" "/home/$user_name/repos"
}

ensure_ssh_server() {
  ssh-keygen -A

  ssh-keygen -t ed25519 -N '' -f "$HOME/.ssh/alpha_key" >/dev/null
  ssh-keygen -t ed25519 -N '' -f "$HOME/.ssh/beta_key" >/dev/null

  create_remote_user "gitalpha" "$HOME/.ssh/alpha_key"
  create_remote_user "gitbeta" "$HOME/.ssh/beta_key"

  runuser -u gitalpha -- git init --bare /home/gitalpha/repos/alpha.git >/dev/null
  runuser -u gitbeta -- git init --bare /home/gitbeta/repos/beta.git >/dev/null

  cat > "$HOME/.ssh/config" <<EOF
Host git-alpha
  HostName 127.0.0.1
  Port 2222
  User gitalpha
  IdentityFile $HOME/.ssh/alpha_key
  IdentitiesOnly yes
  StrictHostKeyChecking no
  UserKnownHostsFile /dev/null

Host git-beta
  HostName 127.0.0.1
  Port 2222
  User gitbeta
  IdentityFile $HOME/.ssh/beta_key
  IdentitiesOnly yes
  StrictHostKeyChecking no
  UserKnownHostsFile /dev/null
EOF
  chmod 600 "$HOME/.ssh/config"

  cat > /tmp/committy-sshd-config <<'EOF'
Port 2222
ListenAddress 127.0.0.1
HostKey /etc/ssh/ssh_host_ed25519_key
HostKey /etc/ssh/ssh_host_rsa_key
PasswordAuthentication no
PermitRootLogin no
PubkeyAuthentication yes
UsePAM no
PidFile /run/sshd_committy_e2e.pid
AuthorizedKeysFile .ssh/authorized_keys
Subsystem sftp internal-sftp
EOF

  mkdir -p /run/sshd
  /usr/sbin/sshd -f /tmp/committy-sshd-config
}

assert_signed_commit() {
  local repo_path="$1"
  run_git "$repo_path" verify-commit HEAD >/dev/null
}

assert_ssh_signed_commit() {
  local repo_path="$1"
  run_git "$repo_path" \
    -c gpg.format=ssh \
    -c "gpg.ssh.allowedSignersFile=$HOME/.ssh/allowed_signers" \
    verify-commit HEAD >/dev/null
}

assert_signed_tag() {
  local repo_path="$1"
  local tag_name="$2"
  run_git "$repo_path" tag -v "$tag_name" >/dev/null
}

assert_remote_has_tag() {
  local repo_path="$1"
  local tag_name="$2"
  run_git "$repo_path" ls-remote --tags origin | grep -q "refs/tags/${tag_name}$"
}

exercise_repo() {
  local repo_name="$1"
  local remote_host="$2"
  local worktree="/tmp/${repo_name}-worktree"
  local remote_path
  local tag_name="v0.1.0"

  case "$repo_name" in
    alpha) remote_path="/home/gitalpha/repos/alpha.git" ;;
    beta) remote_path="/home/gitbeta/repos/beta.git" ;;
    *)
      echo "unknown repo name: $repo_name" >&2
      exit 1
      ;;
  esac

  rm -rf "$worktree"
  mkdir -p "$worktree"
  run_git "$worktree" init >/dev/null
  run_git "$worktree" remote add origin "${remote_host}:${remote_path}"

  printf '%s\n' "hello from ${repo_name}" > "$worktree/README.md"
  run_git "$worktree" add README.md

  "$committy_bin" --non-interactive commit \
    --repo-path "$worktree" \
    --type feat \
    --message "add ${repo_name} smoke coverage"

  assert_signed_commit "$worktree"

  "$committy_bin" --non-interactive tag \
    --repo-path "$worktree" \
    --name "$tag_name" \
    --publish \
    --confirm-publish

  assert_signed_tag "$worktree" "$tag_name"
  assert_remote_has_tag "$worktree" "$tag_name"

  printf '%s\n' "follow-up for ${repo_name}" >> "$worktree/README.md"
  run_git "$worktree" add README.md

  "$committy_bin" --non-interactive commit \
    --repo-path "$worktree" \
    --type fix \
    --message "prepare ${repo_name} fetch validation" \
    --git-config "gpg.format=ssh" \
    --git-config "user.signingkey=$HOME/.ssh/signing_key.pub" \
    --git-config "gpg.ssh.allowedSignersFile=$HOME/.ssh/allowed_signers"

  assert_ssh_signed_commit "$worktree"

  "$committy_bin" --non-interactive tag \
    --repo-path "$worktree" \
    --fetch \
    --dry-run \
    --output json >/tmp/"${repo_name}"-fetch.json

  grep -q '"command":"tag"' /tmp/"${repo_name}"-fetch.json
}

main() {
  log "configuring signing environment"
  ensure_signing_config

  log "starting local ssh server with two distinct identities"
  ensure_ssh_server

  log "verifying signed commit/tag flow with first ssh identity"
  exercise_repo "alpha" "git-alpha"

  log "verifying signed commit/tag flow with second ssh identity"
  exercise_repo "beta" "git-beta"

  log "native git signing and multi-identity ssh verification passed"
}

main "$@"
