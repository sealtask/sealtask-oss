# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_sealtask_global_optspecs
    string join \n api-url= storage-origin= json format= color= pager= no-pager progress= q/quiet non-interactive v debug connect-timeout= read-timeout= request-timeout= retry= offline profile= config-dir= serve-unlock-daemon= h/help V/version
end

function __fish_sealtask_needs_command
    # Figure out if the current invocation already has a command.
    set -l cmd (commandline -opc)
    set -e cmd[1]
    argparse -s (__fish_sealtask_global_optspecs) -- $cmd 2>/dev/null
    or return
    if set -q argv[1]
        # Also print the command, so this can be used to figure out what it is.
        echo $argv[1]
        return 1
    end
    return 0
end

function __fish_sealtask_using_subcommand
    set -l cmd (__fish_sealtask_needs_command)
    test -z "$cmd"
    and return 1
    contains -- $cmd[1] $argv
end

complete -c sealtask -n "__fish_sealtask_needs_command" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_needs_command" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_needs_command" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_needs_command" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_needs_command" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_needs_command" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_needs_command" -l serve-unlock-daemon -r -F
complete -c sealtask -n "__fish_sealtask_needs_command" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_needs_command" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_needs_command" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_needs_command" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_needs_command" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_needs_command" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_needs_command" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_needs_command" -s h -l help -d 'Print help'
complete -c sealtask -n "__fish_sealtask_needs_command" -s V -l version -d 'Print version'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "completion" -d 'Generate a shell completion script without reading configuration or credentials'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "man" -d 'Render a manual page for the root command or a nested command path'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "info" -d 'Show machine-readable CLI capabilities and contract versions'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "schema" -d 'Describe commands and arguments as human help or versioned JSON'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "auth" -d 'Authenticate, inspect the session, and manage local unlock state'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "me" -d 'Show the current authenticated user'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "pick" -d 'Choose or resolve a project to activate, or interactively print a task selector'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "projects" -d 'List, inspect, archive, or restore projects and inspect saved context'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "lists" -d 'List, inspect, archive, or restore projects and inspect saved context'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "tasks" -d 'List, inspect, create, update, move, or delete tasks'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "stats" -d 'Show current dashboard task counts'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "activity" -d 'Inspect or continuously follow recent account activity'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "browse" -d 'Browse cached or live decrypted projects and tasks in a private TTY'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "cache" -d 'Inspect, verify, or clear the encrypted local read cache'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "batch" -d 'Validate, execute, and safely resume task mutations from JSON Lines'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "doctor" -d 'Diagnose local state, authentication, unlock, and API connectivity'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "config" -d 'Inspect resolved operator configuration'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "profile" -d 'List profiles or change the persisted default profile'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "inspect"
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "comments" -d 'List, create, update, or delete task comments'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "notes" -d 'List, inspect, create, update, or delete encrypted notes'
complete -c sealtask -n "__fish_sealtask_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand completion" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l output-dir -d 'Generate the root and every visible subcommand manual beneath this directory' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand man" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand info" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand schema" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "login" -d 'Sign in with an email and password, optionally completing MFA'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "unlock" -d 'Unlock workspace data in memory for a bounded session'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "lock" -d 'Lock workspace data and stop the in-memory unlock session'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "keychain" -d 'Store or clear this profile\'s saved unlock key'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "logout" -d 'Revoke the remote session and clear this profile\'s local credentials'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "status" -d 'Inspect sign-in, token expiry, workspace-data, and saved-key state'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and not __fish_seen_subcommand_from login unlock lock keychain logout status help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l email -d 'Account email. Required with --non-interactive' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l password-stdin -d 'Read login input from stdin: trimmed password on line 1 and optional exact authenticator or backup code on line 2'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from login" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l ttl -d 'Human duration before the memory-only unlock expires (for example 30m or 8h)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l ttl-seconds -d 'Number of seconds before the memory-only unlock expires (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l password-stdin -d 'Read the account password from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from unlock" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from lock" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -f -a "store" -d 'Save this profile\'s unlock key in the platform keychain'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -f -a "clear" -d 'Remove this profile\'s saved unlock key'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from keychain" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from logout" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "login" -d 'Sign in with an email and password, optionally completing MFA'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "unlock" -d 'Unlock workspace data in memory for a bounded session'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "lock" -d 'Lock workspace data and stop the in-memory unlock session'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "keychain" -d 'Store or clear this profile\'s saved unlock key'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "logout" -d 'Revoke the remote session and clear this profile\'s local credentials'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "status" -d 'Inspect sign-in, token expiry, workspace-data, and saved-key state'
complete -c sealtask -n "__fish_sealtask_using_subcommand auth; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand me" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -f -a "project" -d 'Pick or resolve a project and save it as current'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -f -a "task" -d 'Pick a task in the selected/current project'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and not __fish_seen_subcommand_from project task help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l scope -d 'Save for this directory or as the active profile\'s global fallback' -r -f -a "local\t''
global\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l include-archived -d 'Include archived projects when printing a selector'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l print-selector -d 'Print only a reusable selector without changing project context'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from project" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l include-completed -d 'Include completed tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l include-archived -d 'Include archived tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from task" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from help" -f -a "project" -d 'Pick or resolve a project and save it as current'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from help" -f -a "task" -d 'Pick a task in the selected/current project'
complete -c sealtask -n "__fish_sealtask_using_subcommand pick; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l verbose
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l include-archived -d 'Include archived projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "list" -d 'List accessible projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "get" -d 'Show one decrypted project'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "archive" -d 'Archive a project and make it read-only'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "unarchive" -d 'Restore an archived project'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "current" -d 'Show the effective current project without accessing the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "clear" -d 'Clear one saved context layer while preserving other fallback layers'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "sections" -d 'Discover sections in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "audit" -d 'Show a bounded page of safe project audit metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l details -d 'Print expanded human-readable project details'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l include-archived -d 'Include archived projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from archive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from unarchive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l scope -d 'Inspect only the nearest local layer or the active profile\'s global fallback' -r -f -a "local\t''
global\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from current" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l scope -d 'Clear the nearest local layer or the active profile\'s global fallback' -r -f -a "local\t''
global\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from clear" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -f -a "list" -d 'List normalized project sections and their IDs'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from sections" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l cursor -d 'Fetch entries older than this audit-event UUID' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l limit -d 'Maximum number of audit entries to return' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l password-stdin -d 'Read the account password from stdin when project-name resolution needs an unlock'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from audit" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "list" -d 'List accessible projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "get" -d 'Show one decrypted project'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "archive" -d 'Archive a project and make it read-only'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "unarchive" -d 'Restore an archived project'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "current" -d 'Show the effective current project without accessing the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "clear" -d 'Clear one saved context layer while preserving other fallback layers'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "sections" -d 'Discover sections in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "audit" -d 'Show a bounded page of safe project audit metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand projects; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l verbose
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l include-archived -d 'Include archived projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "list" -d 'List accessible projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "get" -d 'Show one decrypted project'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "archive" -d 'Archive a project and make it read-only'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "unarchive" -d 'Restore an archived project'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "current" -d 'Show the effective current project without accessing the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "clear" -d 'Clear one saved context layer while preserving other fallback layers'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "sections" -d 'Discover sections in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "audit" -d 'Show a bounded page of safe project audit metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and not __fish_seen_subcommand_from list get archive unarchive current clear sections audit help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l details -d 'Print expanded human-readable project details'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l include-archived -d 'Include archived projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from archive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from unarchive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l scope -d 'Inspect only the nearest local layer or the active profile\'s global fallback' -r -f -a "local\t''
global\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from current" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l scope -d 'Clear the nearest local layer or the active profile\'s global fallback' -r -f -a "local\t''
global\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from clear" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -f -a "list" -d 'List normalized project sections and their IDs'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from sections" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l cursor -d 'Fetch entries older than this audit-event UUID' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l limit -d 'Maximum number of audit entries to return' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l password-stdin -d 'Read the account password from stdin when project-name resolution needs an unlock'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from audit" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "list" -d 'List accessible projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "get" -d 'Show one decrypted project'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "archive" -d 'Archive a project and make it read-only'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "unarchive" -d 'Restore an archived project'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "current" -d 'Show the effective current project without accessing the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "clear" -d 'Clear one saved context layer while preserving other fallback layers'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "sections" -d 'Discover sections in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "audit" -d 'Show a bounded page of safe project audit metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand lists; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "list" -d 'List tasks in the selected/current project, or assigned tasks when none is selected'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "get" -d 'Show one decrypted task, including comments and attachment metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "watch" -d 'Follow authoritative task changes in one project until interrupted'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "create" -d 'Create an encrypted task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "edit" -d 'Edit a task\'s title and Markdown body in your configured editor'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "update" -d 'Patch an encrypted task; omitted fields remain unchanged'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "move" -d 'Move a task to a section or relative position'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "complete" -d 'Move a task to the final section'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "reopen" -d 'Move a task to the first section'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "archive" -d 'Archive a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "unarchive" -d 'Restore an archived task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "delete" -d 'Permanently delete a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "attachments" -d 'Upload, delete, read, or download encrypted task attachments'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and not __fish_seen_subcommand_from list get watch create edit update move complete reopen archive unarchive delete attachments help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l project -d 'Restrict results to a project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l work-list-id -d 'Restrict results to one exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l columns -d 'Select and order human table columns (comma-separated or repeatable)' -r -f -a "id\t''
title\t''
project\t''
project-id\t''
priority\t''
due\t''
status\t''
comments\t''
created\t''
updated\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l sort -d 'Sort text/date/status ascending, priority high-first, or timestamps newest-first' -r -f -a "id\t''
title\t''
project\t''
priority\t''
due\t''
status\t''
created\t''
updated\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l field -d 'Emit one sanitized raw value per task with no headings, totals, or empty-state text' -r -f -a "id\t''
title\t''
url\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l web-url -d 'Browser application origin; valid only with --field url (defaults to the API origin)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l include-completed -d 'Include tasks in completed sections'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l include-archived -d 'Include archived tasks from the selected/current project'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l all -d 'List assigned tasks across all accessible projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l raw
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l project -d 'Restrict results to a project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l work-list-id -d 'Restrict results to one exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l include-completed -d 'Include completed tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l include-archived -d 'Include archived tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from watch" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l title -d 'Plaintext task title' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l body -d 'Plaintext Markdown task body' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l body-file -d 'Read the plaintext Markdown task body from PATH; use \'-\' for stdin' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l priority -d 'Task priority: low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l due-at -d 'Due time as an RFC 3339 timestamp' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l due -d 'Human due date in the project\'s timezone (for example tomorrow or 2026-08-10)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l start-at -d 'Start time as an RFC 3339 timestamp' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l section-id -d 'Initial section UUID' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l section -d 'Initial section name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l idempotency-key -d 'Stable retry key containing at most 128 ASCII letters, digits, \'.\', \'_\', \'-\', or \':\'' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l input-file -d 'Read the complete camelCase task input object from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l edit -d 'Open your configured editor; --title, --body, and --body-file seed its contents'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l input-stdin -d 'Read the complete camelCase task input object from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l dry-run -d 'Resolve, validate, and encrypt the request but do not create the task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from edit" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l title -d 'Replace the task title' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l body -d 'Replace the Markdown body' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l body-file -d 'Read the replacement Markdown task body from PATH; use \'-\' for stdin' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l priority -d 'Set priority to low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l due-at -d 'Set the due time as an RFC 3339 timestamp' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l due -d 'Set a human due date in the project\'s timezone' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l start-at -d 'Set the start time as an RFC 3339 timestamp' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l section-id -d 'Move the task to this section UUID' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l section -d 'Move the task to a section name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l input-file -d 'Read the complete camelCase patch object from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l clear-body -d 'Remove the task body'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l clear-priority -d 'Remove the priority'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l clear-due-at -d 'Remove the due time'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l clear-start-at -d 'Remove the start time'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l clear-section -d 'Remove the explicit section assignment'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l input-stdin -d 'Read the complete camelCase patch object from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l dry-run -d 'Resolve, validate, and encrypt the request but do not update the task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from update" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l section-id -d 'Destination section UUID' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l section -d 'Destination section name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l insert-before-task-id -d 'Place the task immediately before this task UUID' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l before -d 'Place the task immediately before this task title, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from move" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from complete" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from reopen" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from archive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from unarchive" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l input-file -d 'Read an optional audit patch from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l input-stdin -d 'Read an optional audit patch from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l password-stdin -d 'Read the account password from stdin while resolving human selectors'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l yes -d 'Confirm permanent deletion without prompting'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -f -a "upload" -d 'Encrypt and upload a local regular file'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -f -a "delete" -d 'Remove an attachment reference and its encrypted object'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -f -a "read" -d 'Decrypt a text or DOCX attachment and print readable text'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -f -a "download" -d 'Decrypt an attachment and save it beneath the current directory'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from attachments" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "list" -d 'List tasks in the selected/current project, or assigned tasks when none is selected'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "get" -d 'Show one decrypted task, including comments and attachment metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "watch" -d 'Follow authoritative task changes in one project until interrupted'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "create" -d 'Create an encrypted task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "edit" -d 'Edit a task\'s title and Markdown body in your configured editor'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "update" -d 'Patch an encrypted task; omitted fields remain unchanged'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "move" -d 'Move a task to a section or relative position'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "complete" -d 'Move a task to the final section'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "reopen" -d 'Move a task to the first section'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "archive" -d 'Archive a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "unarchive" -d 'Restore an archived task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Permanently delete a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "attachments" -d 'Upload, delete, read, or download encrypted task attachments'
complete -c sealtask -n "__fish_sealtask_using_subcommand tasks; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand stats" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -s h -l help -d 'Print help'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -f -a "follow" -d 'Follow new activity using bounded cursor catch-up polling'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and not __fish_seen_subcommand_from follow help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l interval -d 'Delay between activity polls (for example 2s or 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l since -d 'Emit recent history from this window before following new events' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from follow" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from help" -f -a "follow" -d 'Follow new activity using bounded cursor catch-up polling'
complete -c sealtask -n "__fish_sealtask_using_subcommand activity; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l include-completed -d 'Include completed tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l include-archived -d 'Include archived projects and tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand browse" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -f -a "status" -d 'Show cache presence, mode, size, and modification time without decrypting content'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -f -a "verify" -d 'Authenticate, decrypt, and validate the complete local cache'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -f -a "clear" -d 'Remove the encrypted local cache for the active profile'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and not __fish_seen_subcommand_from status verify clear help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from status" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from verify" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from clear" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from help" -f -a "status" -d 'Show cache presence, mode, size, and modification time without decrypting content'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from help" -f -a "verify" -d 'Authenticate, decrypt, and validate the complete local cache'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from help" -f -a "clear" -d 'Remove the encrypted local cache for the active profile'
complete -c sealtask -n "__fish_sealtask_using_subcommand cache; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -f -a "run" -d 'Run a strict versioned JSONL task-mutation batch'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and not __fish_seen_subcommand_from run help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l input -d 'JSONL input path, or \'-\' to read stdin' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l jobs -d 'Maximum number of unrelated operations in flight' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l checkpoint -d 'Durable resumable checkpoint path (Linux and macOS only)' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l continue-on-error -d 'Keep scheduling independent operations after an operation fails'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l resume -d 'Resume an existing Linux/macOS checkpoint bound to the exact canonical input'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l dry-run -d 'Resolve and prepare every operation without issuing mutations'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from run" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from help" -f -a "run" -d 'Run a strict versioned JSONL task-mutation batch'
complete -c sealtask -n "__fish_sealtask_using_subcommand batch; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l strict -d 'Exit unsuccessfully when any check warns'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l include-keychain -d 'Inspect the platform keychain (may trigger an operating-system prompt)'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand doctor" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -f -a "show" -d 'Show safe configuration values and where they came from'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and not __fish_seen_subcommand_from show help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l resolved -d 'Include resolution sources and defaults'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "show" -d 'Show safe configuration values and where they came from'
complete -c sealtask -n "__fish_sealtask_using_subcommand config; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -f -a "list" -d 'List known local profiles and mark the active one'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -f -a "use" -d 'Persist the default profile for future commands'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and not __fish_seen_subcommand_from list use help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from use" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "list" -d 'List known local profiles and mark the active one'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "use" -d 'Persist the default profile for future commands'
complete -c sealtask -n "__fish_sealtask_using_subcommand profile; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l password-stdin
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand inspect" -s h -l help -d 'Print help'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -f -a "list" -d 'List decrypted comments on a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -f -a "create" -d 'Create an encrypted task comment'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -f -a "update" -d 'Replace an encrypted task comment body'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -f -a "delete" -d 'Permanently delete a task comment'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and not __fish_seen_subcommand_from list create update delete help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l body -d 'Plaintext Markdown comment body' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l body-file -d 'Read the plaintext Markdown comment body from PATH; use \'-\' for stdin' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l input-file -d 'Read the complete camelCase comment input object from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l input-stdin -d 'Read the complete camelCase comment input object from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l comment-id -d 'Comment UUID or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l body -d 'Replacement plaintext Markdown comment body' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l input-file -d 'Read the complete camelCase comment input object from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l input-stdin -d 'Read the complete camelCase comment input object from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from update" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l task-id -d 'Exact task UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l comment-id -d 'Comment UUID or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l input-file -d 'Read an optional audit patch from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l input-stdin -d 'Read an optional audit patch from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l password-stdin -d 'Read the account password from stdin while resolving human selectors'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l yes -d 'Confirm permanent deletion without prompting'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from help" -f -a "list" -d 'List decrypted comments on a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from help" -f -a "create" -d 'Create an encrypted task comment'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from help" -f -a "update" -d 'Replace an encrypted task comment body'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Permanently delete a task comment'
complete -c sealtask -n "__fish_sealtask_using_subcommand comments; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "list" -d 'List decrypted notes in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "get" -d 'Show one decrypted note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "create" -d 'Create an encrypted shared or private note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "edit" -d 'Edit a note\'s title and Markdown body in your configured editor'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "update" -d 'Patch an encrypted note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "delete" -d 'Permanently delete a note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and not __fish_seen_subcommand_from list get create edit update delete help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l note-id -d 'Exact note UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from get" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l title -d 'Plaintext note title' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l body -d 'Plaintext Markdown note body' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l idempotency-key -d 'Stable retry key containing at most 128 ASCII letters, digits, \'.\', \'_\', \'-\', or \':\'' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l input-file -d 'Read the complete camelCase note input object from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l private -d 'Encrypt with a per-note key available only to the current user'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l input-stdin -d 'Read the complete camelCase note input object from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l note-id -d 'Exact note UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from edit" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l note-id -d 'Exact note UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l title -d 'Replace the note title' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l body -d 'Replace the Markdown note body' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l input-file -d 'Read the complete camelCase note patch from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l input-stdin -d 'Read the complete camelCase note patch from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l password-stdin -d 'Read the account password from stdin when no local unlock is available'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from update" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l note-id -d 'Exact note UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l project -d 'Project name, UUID, or unique UUID prefix' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l work-list-id -d 'Exact project UUID (legacy compatibility)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l input-file -d 'Read an optional audit patch from a UTF-8 JSON file' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l api-url -d 'SealTask API base URL' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l storage-origin -d 'Trusted origin for presigned attachment transfers (repeatable)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l format -d 'Select human-readable, finite JSON, or streaming JSON Lines output' -r -f -a "table\t''
json\t''
json-pretty\t''
jsonl\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l color -d 'Control colors in human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l pager -d 'Control paging of long human-readable output' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l progress -d 'Control delayed progress indicators on stderr' -r -f -a "auto\t''
always\t''
never\t''"
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l connect-timeout -d 'Maximum time to establish a control-plane connection (for example 5s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l read-timeout -d 'Maximum idle time while reading a control-plane response (for example 30s)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l request-timeout -d 'Maximum total time for one control-plane request (for example 1m)' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l retry -d 'Retry replay-safe API requests after transient failures' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l profile -d 'Isolate credentials and unlock state under a named profile' -r
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l config-dir -d 'Override the base directory used for credentials and local unlock state' -r -F
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l input-stdin -d 'Read an optional audit patch from stdin'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l password-stdin -d 'Read the account password from stdin while resolving human selectors'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l yes -d 'Confirm permanent deletion without prompting'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l json -d 'Emit compact JSON instead of human-readable output'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l no-pager -d 'Disable paging (equivalent to --pager never)'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -s q -l quiet -d 'Suppress automatic paging, progress, and successful mutation acknowledgements'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l non-interactive -d 'Never prompt; fail with an actionable validation error when input is missing'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -s v -d 'Emit redacted operator telemetry to stderr; repeat for more detail'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l debug -d 'Emit maximum redacted diagnostic telemetry to stderr'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -l offline -d 'Read only from the encrypted local snapshot and never access the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "list" -d 'List decrypted notes in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "get" -d 'Show one decrypted note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "create" -d 'Create an encrypted shared or private note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "edit" -d 'Edit a note\'s title and Markdown body in your configured editor'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "update" -d 'Patch an encrypted note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Permanently delete a note'
complete -c sealtask -n "__fish_sealtask_using_subcommand notes; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "completion" -d 'Generate a shell completion script without reading configuration or credentials'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "man" -d 'Render a manual page for the root command or a nested command path'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "info" -d 'Show machine-readable CLI capabilities and contract versions'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "schema" -d 'Describe commands and arguments as human help or versioned JSON'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "auth" -d 'Authenticate, inspect the session, and manage local unlock state'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "me" -d 'Show the current authenticated user'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "pick" -d 'Choose or resolve a project to activate, or interactively print a task selector'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "projects" -d 'List, inspect, archive, or restore projects and inspect saved context'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "tasks" -d 'List, inspect, create, update, move, or delete tasks'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "stats" -d 'Show current dashboard task counts'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "activity" -d 'Inspect or continuously follow recent account activity'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "browse" -d 'Browse cached or live decrypted projects and tasks in a private TTY'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "cache" -d 'Inspect, verify, or clear the encrypted local read cache'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "batch" -d 'Validate, execute, and safely resume task mutations from JSON Lines'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "doctor" -d 'Diagnose local state, authentication, unlock, and API connectivity'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "config" -d 'Inspect resolved operator configuration'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "profile" -d 'List profiles or change the persisted default profile'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "inspect"
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "comments" -d 'List, create, update, or delete task comments'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "notes" -d 'List, inspect, create, update, or delete encrypted notes'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and not __fish_seen_subcommand_from completion man info schema auth me pick projects tasks stats activity browse cache batch doctor config profile inspect comments notes help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "login" -d 'Sign in with an email and password, optionally completing MFA'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "unlock" -d 'Unlock workspace data in memory for a bounded session'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "lock" -d 'Lock workspace data and stop the in-memory unlock session'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "keychain" -d 'Store or clear this profile\'s saved unlock key'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "logout" -d 'Revoke the remote session and clear this profile\'s local credentials'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from auth" -f -a "status" -d 'Inspect sign-in, token expiry, workspace-data, and saved-key state'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from pick" -f -a "project" -d 'Pick or resolve a project and save it as current'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from pick" -f -a "task" -d 'Pick a task in the selected/current project'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "list" -d 'List accessible projects'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "get" -d 'Show one decrypted project'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "archive" -d 'Archive a project and make it read-only'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "unarchive" -d 'Restore an archived project'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "current" -d 'Show the effective current project without accessing the network'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "clear" -d 'Clear one saved context layer while preserving other fallback layers'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "sections" -d 'Discover sections in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from projects" -f -a "audit" -d 'Show a bounded page of safe project audit metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "list" -d 'List tasks in the selected/current project, or assigned tasks when none is selected'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "get" -d 'Show one decrypted task, including comments and attachment metadata'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "watch" -d 'Follow authoritative task changes in one project until interrupted'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "create" -d 'Create an encrypted task'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "edit" -d 'Edit a task\'s title and Markdown body in your configured editor'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "update" -d 'Patch an encrypted task; omitted fields remain unchanged'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "move" -d 'Move a task to a section or relative position'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "complete" -d 'Move a task to the final section'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "reopen" -d 'Move a task to the first section'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "archive" -d 'Archive a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "unarchive" -d 'Restore an archived task'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "delete" -d 'Permanently delete a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from tasks" -f -a "attachments" -d 'Upload, delete, read, or download encrypted task attachments'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from activity" -f -a "follow" -d 'Follow new activity using bounded cursor catch-up polling'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from cache" -f -a "status" -d 'Show cache presence, mode, size, and modification time without decrypting content'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from cache" -f -a "verify" -d 'Authenticate, decrypt, and validate the complete local cache'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from cache" -f -a "clear" -d 'Remove the encrypted local cache for the active profile'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from batch" -f -a "run" -d 'Run a strict versioned JSONL task-mutation batch'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from config" -f -a "show" -d 'Show safe configuration values and where they came from'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "list" -d 'List known local profiles and mark the active one'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from profile" -f -a "use" -d 'Persist the default profile for future commands'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from comments" -f -a "list" -d 'List decrypted comments on a task'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from comments" -f -a "create" -d 'Create an encrypted task comment'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from comments" -f -a "update" -d 'Replace an encrypted task comment body'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from comments" -f -a "delete" -d 'Permanently delete a task comment'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from notes" -f -a "list" -d 'List decrypted notes in a project'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from notes" -f -a "get" -d 'Show one decrypted note'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from notes" -f -a "create" -d 'Create an encrypted shared or private note'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from notes" -f -a "edit" -d 'Edit a note\'s title and Markdown body in your configured editor'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from notes" -f -a "update" -d 'Patch an encrypted note'
complete -c sealtask -n "__fish_sealtask_using_subcommand help; and __fish_seen_subcommand_from notes" -f -a "delete" -d 'Permanently delete a note'
