function write_foreman_toml($protocol, $tool, $source, $version) {
    Write-Output "writing foreman.toml"
    Write-Output "[tools]" | Out-File -FilePath foreman.toml -Encoding utf8
    Write-Output "$tool = { $protocol = `"$source`", version = `"=$version`" }" | Out-File -FilePath foreman.toml -append -Encoding utf8
}

function create_rojo_files() {
    Write-Output "writing default.project.json"
    Write-Output "{ 
        `"name`": `"test`", 
        `"tree`": { 
            `"`$path`": `"src`"
        } 
    }" | Out-File -FilePath default.project.json -Encoding utf8
}

function setup_rojo() {
    write_foreman_toml github rojo "rojo-rbx/rojo" "7.3.0"
    cargo run --release -- install
    create_rojo_files
}

function kill_process_and_check_delayed() {
    Write-Output "waiting 15 seconds before killing rojo"
    Start-Sleep 15
    Get-Process | Where-Object { $_.Name -eq "rojo" } | Select-Object -First 1 | Stop-Process
    Write-Output "waiting 5 seconds to stop rojo"
    Start-Sleep 5
    check_killed_subprocess
}

function run_rojo_serve_and_kill_process() {
    setup_rojo
    Start-job -ScriptBlock { rojo serve default.project.json }
    kill_process_and_check_delayed
}

function check_killed_subprocess() {
    Write-Output "Checking if process was killed properly"
    $rojo = Get-Process -name "rojo-rbx__rojo-7.3.0" -ErrorAction SilentlyContinue
    if ($rojo) {
        Write-Output "rojo subprocess was not killed properly"
        remove-item foreman.toml
        remove-item default.project.json
        exit 1
    }
    else {
        Write-Output "rojo subprocess was killed properly"
        remove-item foreman.toml
        remove-item default.project.json
        exit 0
    }
}

run_rojo_serve_and_kill_process