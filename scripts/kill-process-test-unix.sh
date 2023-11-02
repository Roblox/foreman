#!/bin/bash

write_foreman_toml () {
    echo "writing foreman.toml"
    echo "[tools]" > foreman.toml
    echo "$2 = { $1 = \"$3\", version = \"=$4\" }" >> foreman.toml
}

create_rojo_files() {
    echo "writing default.project.json"
    echo "{
        \"name\": \"test\",
        \"tree\": {
            \"\$path\": \"src\"
            }
    }" > default.project.json
}

setup_rojo() {
    write_foreman_toml github rojo "rojo-rbx/rojo" "7.3.0"
    foreman install
    create_rojo_files
}

kill_process_and_check_delayed() {
    echo "waiting 5 seconds before killing rojo"
    sleep 5
    ps -ef | grep "rojo serve" | grep -v grep | awk '{print $2}' | xargs kill -INT
    echo "waiting 5 seconds for rojo to be killed"
    sleep 5
    check_killed_subprocess
}

run_rojo_serve_and_kill_process() {
    setup_rojo
    (rojo serve default.project.json) & (kill_process_and_check_delayed)
}

check_killed_subprocess(){
    echo "checking if process was killed properly"
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