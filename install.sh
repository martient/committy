#!/usr/bin/env bash

# MANY THANKS TO: https://github.com/oven-sh/bun for the inspiration of this easy to use install script

set -euo pipefail

platform=$(uname -ms)

Color_Off=''

Red=''
Green=''
Dim=''

Bold_White=''
Bold_Green=''

if [[ -t 1 ]]; then
    Color_Off='\033[0m'

    Red='\033[0;31m'
    Green='\033[0;32m'
    Dim='\033[0;2m'

    Bold_Green='\033[1;32m'
    Bold_White='\033[1m'
fi

error() {
    echo -e "${Red}error${Color_Off}:" "$@" >&2
    exit 1
}

info() {
    echo -e "${Dim}$@ ${Color_Off}"
}

info_bold() {
    echo -e "${Bold_White}$@ ${Color_Off}"
}

success() {
    echo -e "${Green}$@ ${Color_Off}"
}

command -v unzip >/dev/null ||
    error 'unzip is required to install committy'

case $platform in
'Darwin x86_64')
    target=macos-amd64
    ;;
'Darwin arm64')
    target=macos-arm64
    ;;
'Linux aarch64' | 'Linux arm64')
    target=linux-arm64
    ;;
'MINGW64'*)
    target=windows-amd64
    ;;
'Linux x86_64' | *)
    target=linux-amd64
    ;;
esac

case "$target" in
'linux'*)
    if [ -f /etc/alpine-release ]; then
        target="$target-musl"
    fi
    ;;
esac

GITHUB=${GITHUB-"https://github.com"}

github_repo="$GITHUB/martient/committy"

case "$target" in
'darwin-x64'*)
    if [[ $(sysctl -a | grep machdep.cpu | grep AVX2) == '' ]]; then
        target="$target-baseline"
    fi
    ;;
'linux-x64'*)
    # If AVX2 isn't supported, use the -baseline build
    if [[ $(cat /proc/cpuinfo | grep avx2) = '' ]]; then
        target="$target-baseline"
    fi
    ;;
esac

exe_name=committy

if [[ $# = 0 ]]; then
    committy_uri=$github_repo/releases/latest/download/committy-$target.tar.gz
else
    committy_uri=$github_repo/releases/download/$1/committy-$target.tar.gz
fi


install_env=COMMITTY_INSTALL

bin_env=\$$install_env/bin

install_dir=${!install_env:-$HOME/.committy}
bin_dir=$install_dir/bin
exe=$bin_dir/committy

if [[ ! -d $bin_dir ]]; then
    mkdir -p "$bin_dir" ||
        error "Failed to create install directory \"$bin_dir\""
fi

curl --fail --location --progress-bar --output "$exe.tar.gz" "$committy_uri" ||
    error "Failed to download committy from \"$committy_uri\""

tar -xf "$exe.tar.gz" -C "$bin_dir" ||
    error 'Failed to extract committy'

mv "$bin_dir/$exe_name" "$exe" ||
    error 'Failed to move extracted committy to destination'

chmod +x "$exe" ||
    error 'Failed to set permissions on committy executable'

tildify() {
    if [[ $1 = $HOME/* ]]; then
        local replacement=\~/

        echo "${1/$HOME\//$replacement}"
    else
        echo "$1"
    fi
}

success "committy was installed successfully to $Bold_Green$(tildify "$exe")"

if command -v committy >/dev/null; then
    # Install completions, but we don't care if it fails
    IS_COMMITTY_AUTO_UPDATE=true $exe completions &>/dev/null || :

    # echo "Run 'committy --help' to get started"
    exit
fi

refresh_command=''

tilde_bin_dir=$(tildify "$bin_dir")
quoted_install_dir=\"${install_dir//\"/\\\"}\"

if [[ $quoted_install_dir = \"$HOME/* ]]; then
    quoted_install_dir=${quoted_install_dir/$HOME\//\$HOME/}
fi

echo

case $(basename "$SHELL") in
fish)
    # Install completions, but we don't care if it fails
    IS_COMMITTY_AUTO_UPDATE=true SHELL=fish $exe completions &>/dev/null || :

    commands=(
        "set --export $install_env $quoted_install_dir"
        "set --export PATH $bin_env \$PATH"
    )

    fish_config=$HOME/.config/fish/config.fish
    tilde_fish_config=$(tildify "$fish_config")

    if [[ -w $fish_config ]]; then
        {
            echo -e '\n# committy'

            for command in "${commands[@]}"; do
                echo "$command"
            done
        } >>"$fish_config"

        info "Added \"$tilde_bin_dir\" to \$PATH in \"$tilde_fish_config\""

        refresh_command="source $tilde_fish_config"
    else
        echo "Manually add the directory to $tilde_fish_config (or similar):"

        for command in "${commands[@]}"; do
            info_bold "  $command"
        done
    fi
    ;;
zsh)
    IS_COMMITTY_AUTO_UPDATE=true SHELL=zsh $exe completions &>/dev/null || :

    commands=(
        "export $install_env=$quoted_install_dir"
        "export PATH=\"$bin_env:\$PATH\""
    )

    zsh_config=$HOME/.zshrc
    tilde_zsh_config=$(tildify "$zsh_config")

    if [[ -w $zsh_config ]]; then
        {
            echo -e '\n# committy'

            for command in "${commands[@]}"; do
                echo "$command"
            done
        } >>"$zsh_config"

        info "Added \"$tilde_bin_dir\" to \$PATH in \"$tilde_zsh_config\""

        refresh_command="exec $SHELL"
    else
        echo "Manually add the directory to $tilde_zsh_config (or similar):"

        for command in "${commands[@]}"; do
            info_bold "  $command"
        done
    fi
    ;;
bash)
    # Install completions, but we don't care if it fails
    IS_COMMITTY_AUTO_UPDATE=true SHELL=bash $exe completions &>/dev/null || :

    commands=(
        "export $install_env=$quoted_install_dir"
        "export PATH=\"$bin_env:\$PATH\""
    )

    bash_configs=(
        "$HOME/.bashrc"
        "$HOME/.bash_profile"
    )

    if [[ ${XDG_CONFIG_HOME:-} ]]; then
        bash_configs+=(
            "$XDG_CONFIG_HOME/.bash_profile"
            "$XDG_CONFIG_HOME/.bashrc"
            "$XDG_CONFIG_HOME/bash_profile"
            "$XDG_CONFIG_HOME/bashrc"
        )
    fi

    set_manually=true
    for bash_config in "${bash_configs[@]}"; do
        tilde_bash_config=$(tildify "$bash_config")

        if [[ -w $bash_config ]]; then
            {
                echo -e '\n# committy'

                for command in "${commands[@]}"; do
                    echo "$command"
                done
            } >>"$bash_config"

            info "Added \"$tilde_bin_dir\" to \$PATH in \"$tilde_bash_config\""

            refresh_command="source $bash_config"
            set_manually=false
            break
        fi
    done

    if [[ $set_manually = true ]]; then
        echo "Manually add the directory to $tilde_bash_config (or similar):"

        for command in "${commands[@]}"; do
            info_bold "  $command"
        done
    fi
    ;;
*)
    echo 'Manually add the directory to ~/.bashrc (or similar):'
    info_bold "  export $install_env=$quoted_install_dir"
    info_bold "  export PATH=\"$bin_env:\$PATH\""
    ;;
esac

echo
info "To get started, run:"
echo

if [[ $refresh_command ]]; then
    info_bold "  $refresh_command"
fi

info_bold "  committy --help"