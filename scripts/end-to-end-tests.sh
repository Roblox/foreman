#!/bin/bash

set -e

verify_tool_version () {
    echo "verify tool" $1
    TOOL_VERSION=$($1 -V)

    if [[ $TOOL_VERSION != "$1 $2" ]]; then
        echo "version did not match:" $TOOL_VERSION
        exit 1
    else
        echo $1 "is there!"
    fi
}

write_foreman_toml () {
    echo "[tools]" > foreman.toml
    echo "$2 = { $1 = \"$3\", version = \"=$4\" }" >> foreman.toml
}

verify_github_tool () {
    write_foreman_toml github $1 $2 $3
    foreman install
    verify_tool_version $1 $3
    rm foreman.toml

    # for compatibility, verify that `source` also works
    write_foreman_toml source $1 $2 $3
    foreman install
    verify_tool_version $1 $3
    rm foreman.toml
}

verify_gitlab_tool () {
    write_foreman_toml gitlab $1 $2 $3
    foreman install
    verify_tool_version $1 $3
    rm foreman.toml
}

verify_github_tool Rojo "rojo-rbx/rojo" "6.0.0"
verify_github_tool remodel "rojo-rbx/remodel" "0.9.1"
verify_github_tool stylua "JohnnyMorganz/stylua" "0.11.3"

verify_gitlab_tool darklua "seaofvoices/darklua" "0.7.0"
