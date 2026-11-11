// completions.rs
// Shell completion generator for Bash, Zsh, Fish, and PowerShell.

/// Generates a native shell completion script for the requested shell.
#[must_use]
pub fn generate_completions(shell: &str) -> String {
    match shell.to_lowercase().as_str() {
        "bash" => generate_bash_completions(),
        "zsh" => generate_zsh_completions(),
        "fish" => generate_fish_completions(),
        "powershell" | "pwsh" => generate_powershell_completions(),
        _ => format!("# Unsupported shell: {shell}. Supported: bash, zsh, fish, powershell\n"),
    }
}

fn generate_bash_completions() -> String {
    r#"# Bash completion for umcf
_umcf() {
    local cur prev words cword
    _init_completion || return

    local commands="daemon cluster job ai render media stream storage market doctor completions"
    local daemon_cmds="start stop status logs doctor"
    local cluster_cmds="nodes pair join leave top"
    local job_cmds="submit list status logs cancel watch"
    local ai_cmds="models run chat serve"
    local storage_cmds="put get ls pin quota gc"

    if [ $cword -eq 1 ]; then
        COMPREPLY=( $(compgen -W "${commands}" -- ${cur}) )
        return 0
    fi

    case "${words[1]}" in
        daemon)
            COMPREPLY=( $(compgen -W "${daemon_cmds}" -- ${cur}) )
            ;;
        cluster)
            COMPREPLY=( $(compgen -W "${cluster_cmds}" -- ${cur}) )
            ;;
        job)
            COMPREPLY=( $(compgen -W "${job_cmds}" -- ${cur}) )
            ;;
        ai)
            COMPREPLY=( $(compgen -W "${ai_cmds}" -- ${cur}) )
            ;;
        storage)
            COMPREPLY=( $(compgen -W "${storage_cmds}" -- ${cur}) )
            ;;
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell" -- ${cur}) )
            ;;
        *)
            ;;
    esac
}
complete -F _umcf umcf
"#
    .to_string()
}

fn generate_zsh_completions() -> String {
    r#"#compdef umcf
_umcf() {
    local -a commands
    commands=(
        'daemon:Manage background umcfd daemon'
        'cluster:Inspect and manage cluster topology and pairing'
        'job:Submit and track distributed WASM/compute jobs'
        'ai:Manage and run distributed AI/GGUF models'
        'render:Dispatch GPU tile/frame render tasks'
        'media:Segment-parallel video/audio transcoding'
        'stream:Low-latency remote desktop and game streaming'
        'storage:Content-addressed FastCDC chunk store'
        'market:Resource metering and compute credits'
        'doctor:Diagnose hardware accelerators and network health'
        'completions:Generate shell completion script'
    )
    _arguments '1: :->command' '*: :->args'
}
"#
    .to_string()
}

fn generate_fish_completions() -> String {
    r#"# Fish completion for umcf
complete -c umcf -n "__fish_use_subcommand" -a "daemon" -d "Manage umcfd daemon"
complete -c umcf -n "__fish_use_subcommand" -a "cluster" -d "Cluster topology & peers"
complete -c umcf -n "__fish_use_subcommand" -a "job" -d "Distributed jobs"
complete -c umcf -n "__fish_use_subcommand" -a "ai" -d "Distributed LLM inference"
complete -c umcf -n "__fish_use_subcommand" -a "storage" -d "Content-addressed storage"
complete -c umcf -n "__fish_use_subcommand" -a "completions" -d "Generate completions"
"#
    .to_string()
}

fn generate_powershell_completions() -> String {
    r#"# PowerShell completion for umcf
Register-ArgumentCompleter -Native -CommandName umcf -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $commands = @(
        [CompletionResult]::new("daemon", "daemon", [CompletionResultType]::ParameterValue, "Manage umcfd daemon")
        [CompletionResult]::new("cluster", "cluster", [CompletionResultType]::ParameterValue, "Cluster topology & peers")
        [CompletionResult]::new("job", "job", [CompletionResultType]::ParameterValue, "Distributed jobs")
        [CompletionResult]::new("ai", "ai", [CompletionResultType]::ParameterValue, "Distributed LLM inference")
        [CompletionResult]::new("render", "render", [CompletionResultType]::ParameterValue, "GPU rendering")
        [CompletionResult]::new("media", "media", [CompletionResultType]::ParameterValue, "Media transcoding")
        [CompletionResult]::new("stream", "stream", [CompletionResultType]::ParameterValue, "Remote desktop streaming")
        [CompletionResult]::new("storage", "storage", [CompletionResultType]::ParameterValue, "Content-addressed storage")
        [CompletionResult]::new("market", "market", [CompletionResultType]::ParameterValue, "Compute accounting")
        [CompletionResult]::new("doctor", "doctor", [CompletionResultType]::ParameterValue, "System hardware diagnostics")
        [CompletionResult]::new("completions", "completions", [CompletionResultType]::ParameterValue, "Generate shell completions")
    )
    $commands | Where-Object { $_.CompletionText -like "$wordToComplete*" }
}
"#
    .to_string()
}
