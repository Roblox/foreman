#!/bin/bash

write_foreman_toml () {
    echo "[tools]" > foreman.toml
    echo "$2 = { $1 = \"$3\", version = \"=$4\" }" >> foreman.toml
}

create_rojo_files() {
    echo "{
        \"name\": \"test\",
        \"tree\": {
            \"\$path\": \"src\"
            }
    }" > default.project.json
}

setup_rojo() {
    write_foreman_toml github rojo "rojo-rbx/rojo" "7.3.0"
    cargo run --release -- install
    create_rojo_files
}

delay_kill_process_and_check() {
    echo "waiting 5 seconds before killing rojo"
    sleep 5
    ps -ef | grep "rojo serve" | grep -v grep
    ps -ef | grep "rojo serve" | grep -v grep | awk '{print $2}' | xargs kill -INT
    ps -ef | grep "rojo serve" | grep -v grep
    check_killed_subprocess
}

run_rojo_serve_and_kill_process() {
    setup_rojo
    (rojo serve default.project.json) & (delay_kill_process_and_check)
}

check_killed_subprocess(){
    if ps -ef | grep "rojo" | grep -v grep 
    then 
        echo "rojo subprocess was not killed properly"
        rm foreman.toml
        rm default.project.json
        exit 1
    else
        echo "rojo subprocess was killed properly"
        rm foreman.toml
        rm default.project.json
        exit 0
    fi
}

run_rojo_serve_and_kill_process