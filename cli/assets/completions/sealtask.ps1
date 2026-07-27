
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'sealtask' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'sealtask'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'sealtask' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--serve-unlock-daemon', '--serve-unlock-daemon', [CompletionResultType]::ParameterName, 'serve-unlock-daemon')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate a shell completion script without reading configuration or credentials')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Render a manual page for the root command or a nested command path')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show machine-readable CLI capabilities and contract versions')
            [CompletionResult]::new('schema', 'schema', [CompletionResultType]::ParameterValue, 'Describe commands and arguments as human help or versioned JSON')
            [CompletionResult]::new('auth', 'auth', [CompletionResultType]::ParameterValue, 'Authenticate, inspect the session, and manage local unlock state')
            [CompletionResult]::new('me', 'me', [CompletionResultType]::ParameterValue, 'Show the current authenticated user')
            [CompletionResult]::new('pick', 'pick', [CompletionResultType]::ParameterValue, 'Fuzzy-pick an entity while printing only a reusable opaque selector')
            [CompletionResult]::new('projects', 'projects', [CompletionResultType]::ParameterValue, 'List, inspect, select, archive, or restore projects')
            [CompletionResult]::new('lists', 'lists', [CompletionResultType]::ParameterValue, 'List, inspect, select, archive, or restore projects')
            [CompletionResult]::new('tasks', 'tasks', [CompletionResultType]::ParameterValue, 'List, inspect, create, update, move, or delete tasks')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show current dashboard task counts')
            [CompletionResult]::new('activity', 'activity', [CompletionResultType]::ParameterValue, 'Inspect or continuously follow recent account activity')
            [CompletionResult]::new('browse', 'browse', [CompletionResultType]::ParameterValue, 'Browse cached or live decrypted projects and tasks in a private TTY')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Inspect, verify, or clear the encrypted local read cache')
            [CompletionResult]::new('batch', 'batch', [CompletionResultType]::ParameterValue, 'Validate, execute, and safely resume task mutations from JSON Lines')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose local state, authentication, unlock, and API connectivity')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Inspect resolved operator configuration')
            [CompletionResult]::new('profile', 'profile', [CompletionResultType]::ParameterValue, 'List profiles or change the persisted default profile')
            [CompletionResult]::new('inspect', 'inspect', [CompletionResultType]::ParameterValue, 'inspect')
            [CompletionResult]::new('comments', 'comments', [CompletionResultType]::ParameterValue, 'List, create, update, or delete task comments')
            [CompletionResult]::new('notes', 'notes', [CompletionResultType]::ParameterValue, 'List, inspect, create, update, or delete encrypted notes')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;completion' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;man' {
            [CompletionResult]::new('--output-dir', '--output-dir', [CompletionResultType]::ParameterName, 'Generate the root and every visible subcommand manual beneath this directory')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;info' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;schema' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Sign in with an email and password, optionally completing MFA')
            [CompletionResult]::new('unlock', 'unlock', [CompletionResultType]::ParameterValue, 'Unlock workspace data in memory for a bounded session')
            [CompletionResult]::new('lock', 'lock', [CompletionResultType]::ParameterValue, 'Lock workspace data and stop the in-memory unlock session')
            [CompletionResult]::new('keychain', 'keychain', [CompletionResultType]::ParameterValue, 'Store or clear this profile''s saved unlock key')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Revoke the remote session and clear this profile''s local credentials')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Inspect sign-in, token expiry, workspace-data, and saved-key state')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;auth;login' {
            [CompletionResult]::new('--email', '--email', [CompletionResultType]::ParameterName, 'Account email. Required with --non-interactive')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read login input from stdin: trimmed password on line 1 and optional exact authenticator or backup code on line 2')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;unlock' {
            [CompletionResult]::new('--ttl', '--ttl', [CompletionResultType]::ParameterName, 'Human duration before the memory-only unlock expires (for example 30m or 8h)')
            [CompletionResult]::new('--ttl-seconds', '--ttl-seconds', [CompletionResultType]::ParameterName, 'Number of seconds before the memory-only unlock expires (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;lock' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;keychain' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('store', 'store', [CompletionResultType]::ParameterValue, 'Save this profile''s unlock key in the platform keychain')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove this profile''s saved unlock key')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;auth;keychain;store' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;keychain;clear' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;keychain;help' {
            [CompletionResult]::new('store', 'store', [CompletionResultType]::ParameterValue, 'Save this profile''s unlock key in the platform keychain')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove this profile''s saved unlock key')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;auth;keychain;help;store' {
            break
        }
        'sealtask;auth;keychain;help;clear' {
            break
        }
        'sealtask;auth;keychain;help;help' {
            break
        }
        'sealtask;auth;logout' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;status' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;auth;help' {
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Sign in with an email and password, optionally completing MFA')
            [CompletionResult]::new('unlock', 'unlock', [CompletionResultType]::ParameterValue, 'Unlock workspace data in memory for a bounded session')
            [CompletionResult]::new('lock', 'lock', [CompletionResultType]::ParameterValue, 'Lock workspace data and stop the in-memory unlock session')
            [CompletionResult]::new('keychain', 'keychain', [CompletionResultType]::ParameterValue, 'Store or clear this profile''s saved unlock key')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Revoke the remote session and clear this profile''s local credentials')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Inspect sign-in, token expiry, workspace-data, and saved-key state')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;auth;help;login' {
            break
        }
        'sealtask;auth;help;unlock' {
            break
        }
        'sealtask;auth;help;lock' {
            break
        }
        'sealtask;auth;help;keychain' {
            [CompletionResult]::new('store', 'store', [CompletionResultType]::ParameterValue, 'Save this profile''s unlock key in the platform keychain')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove this profile''s saved unlock key')
            break
        }
        'sealtask;auth;help;keychain;store' {
            break
        }
        'sealtask;auth;help;keychain;clear' {
            break
        }
        'sealtask;auth;help;logout' {
            break
        }
        'sealtask;auth;help;status' {
            break
        }
        'sealtask;auth;help;help' {
            break
        }
        'sealtask;me' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;pick' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('project', 'project', [CompletionResultType]::ParameterValue, 'Pick an accessible project')
            [CompletionResult]::new('task', 'task', [CompletionResultType]::ParameterValue, 'Pick a task in the selected/current project')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;pick;project' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived projects')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;pick;task' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--include-completed', '--include-completed', [CompletionResultType]::ParameterName, 'Include completed tasks')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived tasks')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;pick;help' {
            [CompletionResult]::new('project', 'project', [CompletionResultType]::ParameterValue, 'Pick an accessible project')
            [CompletionResult]::new('task', 'task', [CompletionResultType]::ParameterValue, 'Pick a task in the selected/current project')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;pick;help;project' {
            break
        }
        'sealtask;pick;help;task' {
            break
        }
        'sealtask;pick;help;help' {
            break
        }
        'sealtask;projects' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived projects')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List accessible projects')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted project')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a project and make it read-only')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived project')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Save a project as the current project for this profile')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Show the saved current project without accessing the network')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear the saved current project')
            [CompletionResult]::new('sections', 'sections', [CompletionResultType]::ParameterValue, 'Discover sections in a project')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Show a bounded page of safe project audit metadata')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;lists' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'verbose')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived projects')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List accessible projects')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted project')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a project and make it read-only')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived project')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Save a project as the current project for this profile')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Show the saved current project without accessing the network')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear the saved current project')
            [CompletionResult]::new('sections', 'sections', [CompletionResultType]::ParameterValue, 'Discover sections in a project')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Show a bounded page of safe project audit metadata')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;projects;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--details', '--details', [CompletionResultType]::ParameterName, 'Print expanded human-readable project details')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived projects')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--details', '--details', [CompletionResultType]::ParameterName, 'Print expanded human-readable project details')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived projects')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;get' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;get' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;archive' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;archive' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;unarchive' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;unarchive' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;use' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;use' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;current' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;current' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;clear' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;clear' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;sections' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;projects;sections;list' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;sections;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;projects;sections;help;list' {
            break
        }
        'sealtask;projects;sections;help;help' {
            break
        }
        'sealtask;lists;sections' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;lists;sections;list' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;sections;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;lists;sections;help;list' {
            break
        }
        'sealtask;lists;sections;help;help' {
            break
        }
        'sealtask;projects;audit' {
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--cursor', '--cursor', [CompletionResultType]::ParameterName, 'Fetch entries older than this audit-event UUID')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of audit entries to return')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when project-name resolution needs an unlock')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;lists;audit' {
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--cursor', '--cursor', [CompletionResultType]::ParameterName, 'Fetch entries older than this audit-event UUID')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum number of audit entries to return')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when project-name resolution needs an unlock')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;projects;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List accessible projects')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted project')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a project and make it read-only')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived project')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Save a project as the current project for this profile')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Show the saved current project without accessing the network')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear the saved current project')
            [CompletionResult]::new('sections', 'sections', [CompletionResultType]::ParameterValue, 'Discover sections in a project')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Show a bounded page of safe project audit metadata')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;projects;help;list' {
            break
        }
        'sealtask;projects;help;get' {
            break
        }
        'sealtask;projects;help;archive' {
            break
        }
        'sealtask;projects;help;unarchive' {
            break
        }
        'sealtask;projects;help;use' {
            break
        }
        'sealtask;projects;help;current' {
            break
        }
        'sealtask;projects;help;clear' {
            break
        }
        'sealtask;projects;help;sections' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            break
        }
        'sealtask;projects;help;sections;list' {
            break
        }
        'sealtask;projects;help;audit' {
            break
        }
        'sealtask;projects;help;help' {
            break
        }
        'sealtask;lists;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List accessible projects')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted project')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a project and make it read-only')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived project')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Save a project as the current project for this profile')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Show the saved current project without accessing the network')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear the saved current project')
            [CompletionResult]::new('sections', 'sections', [CompletionResultType]::ParameterValue, 'Discover sections in a project')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Show a bounded page of safe project audit metadata')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;lists;help;list' {
            break
        }
        'sealtask;lists;help;get' {
            break
        }
        'sealtask;lists;help;archive' {
            break
        }
        'sealtask;lists;help;unarchive' {
            break
        }
        'sealtask;lists;help;use' {
            break
        }
        'sealtask;lists;help;current' {
            break
        }
        'sealtask;lists;help;clear' {
            break
        }
        'sealtask;lists;help;sections' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            break
        }
        'sealtask;lists;help;sections;list' {
            break
        }
        'sealtask;lists;help;audit' {
            break
        }
        'sealtask;lists;help;help' {
            break
        }
        'sealtask;tasks' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List tasks in the selected/current project, or assigned tasks when none is selected')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted task, including comments and attachment metadata')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'Follow authoritative task changes in one project until interrupted')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted task')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit a task''s title and Markdown body in your configured editor')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Patch an encrypted task; omitted fields remain unchanged')
            [CompletionResult]::new('move', 'move', [CompletionResultType]::ParameterValue, 'Move a task to a section or relative position')
            [CompletionResult]::new('complete', 'complete', [CompletionResultType]::ParameterValue, 'Move a task to the final section')
            [CompletionResult]::new('reopen', 'reopen', [CompletionResultType]::ParameterValue, 'Move a task to the first section')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a task')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived task')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a task')
            [CompletionResult]::new('attachments', 'attachments', [CompletionResultType]::ParameterValue, 'Upload, delete, read, or download encrypted task attachments')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;tasks;list' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Restrict results to a project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Restrict results to one exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--columns', '--columns', [CompletionResultType]::ParameterName, 'Select and order human table columns (comma-separated or repeatable)')
            [CompletionResult]::new('--sort', '--sort', [CompletionResultType]::ParameterName, 'Sort text/date/status ascending, priority high-first, or timestamps newest-first')
            [CompletionResult]::new('--field', '--field', [CompletionResultType]::ParameterName, 'Emit one sanitized raw value per task with no headings, totals, or empty-state text')
            [CompletionResult]::new('--web-url', '--web-url', [CompletionResultType]::ParameterName, 'Browser application origin; valid only with --field url (defaults to the API origin)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--include-completed', '--include-completed', [CompletionResultType]::ParameterName, 'Include tasks in completed sections')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived tasks from the selected/current project')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'List assigned tasks across all accessible projects')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;get' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--raw', '--raw', [CompletionResultType]::ParameterName, 'raw')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;watch' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Restrict results to a project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Restrict results to one exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--include-completed', '--include-completed', [CompletionResultType]::ParameterName, 'Include completed tasks')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived tasks')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;create' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--title', '--title', [CompletionResultType]::ParameterName, 'Plaintext task title')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Plaintext Markdown task body')
            [CompletionResult]::new('--body-file', '--body-file', [CompletionResultType]::ParameterName, 'Read the plaintext Markdown task body from PATH; use ''-'' for stdin')
            [CompletionResult]::new('--priority', '--priority', [CompletionResultType]::ParameterName, 'Task priority: low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8')
            [CompletionResult]::new('--due-at', '--due-at', [CompletionResultType]::ParameterName, 'Due time as an RFC 3339 timestamp')
            [CompletionResult]::new('--due', '--due', [CompletionResultType]::ParameterName, 'Human due date in the project''s timezone (for example tomorrow or 2026-08-10)')
            [CompletionResult]::new('--start-at', '--start-at', [CompletionResultType]::ParameterName, 'Start time as an RFC 3339 timestamp')
            [CompletionResult]::new('--section-id', '--section-id', [CompletionResultType]::ParameterName, 'Initial section UUID')
            [CompletionResult]::new('--section', '--section', [CompletionResultType]::ParameterName, 'Initial section name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--idempotency-key', '--idempotency-key', [CompletionResultType]::ParameterName, 'Stable retry key containing at most 128 ASCII letters, digits, ''.'', ''_'', ''-'', or '':''')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read the complete camelCase task input object from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--edit', '--edit', [CompletionResultType]::ParameterName, 'Open your configured editor; --title, --body, and --body-file seed its contents')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read the complete camelCase task input object from stdin')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Resolve, validate, and encrypt the request but do not create the task')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;edit' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;update' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--title', '--title', [CompletionResultType]::ParameterName, 'Replace the task title')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Replace the Markdown body')
            [CompletionResult]::new('--body-file', '--body-file', [CompletionResultType]::ParameterName, 'Read the replacement Markdown task body from PATH; use ''-'' for stdin')
            [CompletionResult]::new('--priority', '--priority', [CompletionResultType]::ParameterName, 'Set priority to low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8')
            [CompletionResult]::new('--due-at', '--due-at', [CompletionResultType]::ParameterName, 'Set the due time as an RFC 3339 timestamp')
            [CompletionResult]::new('--due', '--due', [CompletionResultType]::ParameterName, 'Set a human due date in the project''s timezone')
            [CompletionResult]::new('--start-at', '--start-at', [CompletionResultType]::ParameterName, 'Set the start time as an RFC 3339 timestamp')
            [CompletionResult]::new('--section-id', '--section-id', [CompletionResultType]::ParameterName, 'Move the task to this section UUID')
            [CompletionResult]::new('--section', '--section', [CompletionResultType]::ParameterName, 'Move the task to a section name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read the complete camelCase patch object from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--clear-body', '--clear-body', [CompletionResultType]::ParameterName, 'Remove the task body')
            [CompletionResult]::new('--clear-priority', '--clear-priority', [CompletionResultType]::ParameterName, 'Remove the priority')
            [CompletionResult]::new('--clear-due-at', '--clear-due-at', [CompletionResultType]::ParameterName, 'Remove the due time')
            [CompletionResult]::new('--clear-start-at', '--clear-start-at', [CompletionResultType]::ParameterName, 'Remove the start time')
            [CompletionResult]::new('--clear-section', '--clear-section', [CompletionResultType]::ParameterName, 'Remove the explicit section assignment')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read the complete camelCase patch object from stdin')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Resolve, validate, and encrypt the request but do not update the task')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;move' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--section-id', '--section-id', [CompletionResultType]::ParameterName, 'Destination section UUID')
            [CompletionResult]::new('--section', '--section', [CompletionResultType]::ParameterName, 'Destination section name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--insert-before-task-id', '--insert-before-task-id', [CompletionResultType]::ParameterName, 'Place the task immediately before this task UUID')
            [CompletionResult]::new('--before', '--before', [CompletionResultType]::ParameterName, 'Place the task immediately before this task title, UUID, or unique UUID prefix')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;complete' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;reopen' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;archive' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;unarchive' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;delete' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read an optional audit patch from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read an optional audit patch from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin while resolving human selectors')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Confirm permanent deletion without prompting')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;attachments' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('upload', 'upload', [CompletionResultType]::ParameterValue, 'Encrypt and upload a local regular file')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Remove an attachment reference and its encrypted object')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Decrypt a text or DOCX attachment and print readable text')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Decrypt an attachment and save it beneath the current directory')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;tasks;attachments;upload' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Current-working-directory-relative regular file to upload (absolute paths, parent traversal, and symlinks are rejected)')
            [CompletionResult]::new('--file-name', '--file-name', [CompletionResultType]::ParameterName, 'Override the attachment file name stored in the encrypted task')
            [CompletionResult]::new('--content-type', '--content-type', [CompletionResultType]::ParameterName, 'Override the detected MIME content type')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;attachments;delete' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--attachment-id', '--attachment-id', [CompletionResultType]::ParameterName, 'Attachment UUID or unique UUID prefix')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Confirm permanent deletion without prompting')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;attachments;read' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--attachment-id', '--attachment-id', [CompletionResultType]::ParameterName, 'Attachment UUID or unique UUID prefix')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;attachments;download' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--attachment-id', '--attachment-id', [CompletionResultType]::ParameterName, 'Attachment UUID or unique UUID prefix')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Current-working-directory-relative output path (absolute paths, parent traversal, and symlinks are rejected)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Replace an existing output file')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;tasks;attachments;help' {
            [CompletionResult]::new('upload', 'upload', [CompletionResultType]::ParameterValue, 'Encrypt and upload a local regular file')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Remove an attachment reference and its encrypted object')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Decrypt a text or DOCX attachment and print readable text')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Decrypt an attachment and save it beneath the current directory')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;tasks;attachments;help;upload' {
            break
        }
        'sealtask;tasks;attachments;help;delete' {
            break
        }
        'sealtask;tasks;attachments;help;read' {
            break
        }
        'sealtask;tasks;attachments;help;download' {
            break
        }
        'sealtask;tasks;attachments;help;help' {
            break
        }
        'sealtask;tasks;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List tasks in the selected/current project, or assigned tasks when none is selected')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted task, including comments and attachment metadata')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'Follow authoritative task changes in one project until interrupted')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted task')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit a task''s title and Markdown body in your configured editor')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Patch an encrypted task; omitted fields remain unchanged')
            [CompletionResult]::new('move', 'move', [CompletionResultType]::ParameterValue, 'Move a task to a section or relative position')
            [CompletionResult]::new('complete', 'complete', [CompletionResultType]::ParameterValue, 'Move a task to the final section')
            [CompletionResult]::new('reopen', 'reopen', [CompletionResultType]::ParameterValue, 'Move a task to the first section')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a task')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived task')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a task')
            [CompletionResult]::new('attachments', 'attachments', [CompletionResultType]::ParameterValue, 'Upload, delete, read, or download encrypted task attachments')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;tasks;help;list' {
            break
        }
        'sealtask;tasks;help;get' {
            break
        }
        'sealtask;tasks;help;watch' {
            break
        }
        'sealtask;tasks;help;create' {
            break
        }
        'sealtask;tasks;help;edit' {
            break
        }
        'sealtask;tasks;help;update' {
            break
        }
        'sealtask;tasks;help;move' {
            break
        }
        'sealtask;tasks;help;complete' {
            break
        }
        'sealtask;tasks;help;reopen' {
            break
        }
        'sealtask;tasks;help;archive' {
            break
        }
        'sealtask;tasks;help;unarchive' {
            break
        }
        'sealtask;tasks;help;delete' {
            break
        }
        'sealtask;tasks;help;attachments' {
            [CompletionResult]::new('upload', 'upload', [CompletionResultType]::ParameterValue, 'Encrypt and upload a local regular file')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Remove an attachment reference and its encrypted object')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Decrypt a text or DOCX attachment and print readable text')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Decrypt an attachment and save it beneath the current directory')
            break
        }
        'sealtask;tasks;help;attachments;upload' {
            break
        }
        'sealtask;tasks;help;attachments;delete' {
            break
        }
        'sealtask;tasks;help;attachments;read' {
            break
        }
        'sealtask;tasks;help;attachments;download' {
            break
        }
        'sealtask;tasks;help;help' {
            break
        }
        'sealtask;stats' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;activity' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('follow', 'follow', [CompletionResultType]::ParameterValue, 'Follow new activity using bounded cursor catch-up polling')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;activity;follow' {
            [CompletionResult]::new('--interval', '--interval', [CompletionResultType]::ParameterName, 'Delay between activity polls (for example 2s or 1m)')
            [CompletionResult]::new('--since', '--since', [CompletionResultType]::ParameterName, 'Emit recent history from this window before following new events')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;activity;help' {
            [CompletionResult]::new('follow', 'follow', [CompletionResultType]::ParameterValue, 'Follow new activity using bounded cursor catch-up polling')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;activity;help;follow' {
            break
        }
        'sealtask;activity;help;help' {
            break
        }
        'sealtask;browse' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--include-completed', '--include-completed', [CompletionResultType]::ParameterName, 'Include completed tasks')
            [CompletionResult]::new('--include-archived', '--include-archived', [CompletionResultType]::ParameterName, 'Include archived projects and tasks')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;cache' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show cache presence, mode, size, and modification time without decrypting content')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Authenticate, decrypt, and validate the complete local cache')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove the encrypted local cache for the active profile')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;cache;status' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;cache;verify' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;cache;clear' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;cache;help' {
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show cache presence, mode, size, and modification time without decrypting content')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Authenticate, decrypt, and validate the complete local cache')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove the encrypted local cache for the active profile')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;cache;help;status' {
            break
        }
        'sealtask;cache;help;verify' {
            break
        }
        'sealtask;cache;help;clear' {
            break
        }
        'sealtask;cache;help;help' {
            break
        }
        'sealtask;batch' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a strict versioned JSONL task-mutation batch')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;batch;run' {
            [CompletionResult]::new('--input', '--input', [CompletionResultType]::ParameterName, 'JSONL input path, or ''-'' to read stdin')
            [CompletionResult]::new('--jobs', '--jobs', [CompletionResultType]::ParameterName, 'Maximum number of unrelated operations in flight')
            [CompletionResult]::new('--checkpoint', '--checkpoint', [CompletionResultType]::ParameterName, 'Durable resumable checkpoint path (Linux and macOS only)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--continue-on-error', '--continue-on-error', [CompletionResultType]::ParameterName, 'Keep scheduling independent operations after an operation fails')
            [CompletionResult]::new('--resume', '--resume', [CompletionResultType]::ParameterName, 'Resume an existing Linux/macOS checkpoint bound to the exact canonical input')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Resolve and prepare every operation without issuing mutations')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;batch;help' {
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a strict versioned JSONL task-mutation batch')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;batch;help;run' {
            break
        }
        'sealtask;batch;help;help' {
            break
        }
        'sealtask;doctor' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--strict', '--strict', [CompletionResultType]::ParameterName, 'Exit unsuccessfully when any check warns')
            [CompletionResult]::new('--include-keychain', '--include-keychain', [CompletionResultType]::ParameterName, 'Inspect the platform keychain (may trigger an operating-system prompt)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;config' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show safe configuration values and where they came from')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;config;show' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--resolved', '--resolved', [CompletionResultType]::ParameterName, 'Include resolution sources and defaults')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;config;help' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show safe configuration values and where they came from')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;config;help;show' {
            break
        }
        'sealtask;config;help;help' {
            break
        }
        'sealtask;profile' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List known local profiles and mark the active one')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Persist the default profile for future commands')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;profile;list' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;profile;use' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;profile;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List known local profiles and mark the active one')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Persist the default profile for future commands')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;profile;help;list' {
            break
        }
        'sealtask;profile;help;use' {
            break
        }
        'sealtask;profile;help;help' {
            break
        }
        'sealtask;inspect' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'password-stdin')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'sealtask;comments' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List decrypted comments on a task')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted task comment')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Replace an encrypted task comment body')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a task comment')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;comments;list' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;comments;create' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Plaintext Markdown comment body')
            [CompletionResult]::new('--body-file', '--body-file', [CompletionResultType]::ParameterName, 'Read the plaintext Markdown comment body from PATH; use ''-'' for stdin')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read the complete camelCase comment input object from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read the complete camelCase comment input object from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;comments;update' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--comment-id', '--comment-id', [CompletionResultType]::ParameterName, 'Comment UUID or unique UUID prefix')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Replacement plaintext Markdown comment body')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read the complete camelCase comment input object from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read the complete camelCase comment input object from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;comments;delete' {
            [CompletionResult]::new('--task-id', '--task-id', [CompletionResultType]::ParameterName, 'Exact task UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--comment-id', '--comment-id', [CompletionResultType]::ParameterName, 'Comment UUID or unique UUID prefix')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read an optional audit patch from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read an optional audit patch from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin while resolving human selectors')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Confirm permanent deletion without prompting')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;comments;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List decrypted comments on a task')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted task comment')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Replace an encrypted task comment body')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a task comment')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;comments;help;list' {
            break
        }
        'sealtask;comments;help;create' {
            break
        }
        'sealtask;comments;help;update' {
            break
        }
        'sealtask;comments;help;delete' {
            break
        }
        'sealtask;comments;help;help' {
            break
        }
        'sealtask;notes' {
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List decrypted notes in a project')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted note')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted shared or private note')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit a note''s title and Markdown body in your configured editor')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Patch an encrypted note')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a note')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;notes;list' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;notes;get' {
            [CompletionResult]::new('--note-id', '--note-id', [CompletionResultType]::ParameterName, 'Exact note UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;notes;create' {
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--title', '--title', [CompletionResultType]::ParameterName, 'Plaintext note title')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Plaintext Markdown note body')
            [CompletionResult]::new('--idempotency-key', '--idempotency-key', [CompletionResultType]::ParameterName, 'Stable retry key containing at most 128 ASCII letters, digits, ''.'', ''_'', ''-'', or '':''')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read the complete camelCase note input object from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--private', '--private', [CompletionResultType]::ParameterName, 'Encrypt with a per-note key available only to the current user')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read the complete camelCase note input object from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;notes;edit' {
            [CompletionResult]::new('--note-id', '--note-id', [CompletionResultType]::ParameterName, 'Exact note UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;notes;update' {
            [CompletionResult]::new('--note-id', '--note-id', [CompletionResultType]::ParameterName, 'Exact note UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--title', '--title', [CompletionResultType]::ParameterName, 'Replace the note title')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Replace the Markdown note body')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read the complete camelCase note patch from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read the complete camelCase note patch from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin when no local unlock is available')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;notes;delete' {
            [CompletionResult]::new('--note-id', '--note-id', [CompletionResultType]::ParameterName, 'Exact note UUID (legacy compatibility)')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name, UUID, or unique UUID prefix')
            [CompletionResult]::new('--work-list-id', '--work-list-id', [CompletionResultType]::ParameterName, 'Exact project UUID (legacy compatibility)')
            [CompletionResult]::new('--input-file', '--input-file', [CompletionResultType]::ParameterName, 'Read an optional audit patch from a UTF-8 JSON file')
            [CompletionResult]::new('--api-url', '--api-url', [CompletionResultType]::ParameterName, 'SealTask API base URL')
            [CompletionResult]::new('--storage-origin', '--storage-origin', [CompletionResultType]::ParameterName, 'Trusted origin for presigned attachment transfers (repeatable)')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Select human-readable, finite JSON, or streaming JSON Lines output')
            [CompletionResult]::new('--color', '--color', [CompletionResultType]::ParameterName, 'Control colors in human-readable output')
            [CompletionResult]::new('--pager', '--pager', [CompletionResultType]::ParameterName, 'Control paging of long human-readable output')
            [CompletionResult]::new('--progress', '--progress', [CompletionResultType]::ParameterName, 'Control delayed progress indicators on stderr')
            [CompletionResult]::new('--connect-timeout', '--connect-timeout', [CompletionResultType]::ParameterName, 'Maximum time to establish a control-plane connection (for example 5s)')
            [CompletionResult]::new('--read-timeout', '--read-timeout', [CompletionResultType]::ParameterName, 'Maximum idle time while reading a control-plane response (for example 30s)')
            [CompletionResult]::new('--request-timeout', '--request-timeout', [CompletionResultType]::ParameterName, 'Maximum total time for one control-plane request (for example 1m)')
            [CompletionResult]::new('--retry', '--retry', [CompletionResultType]::ParameterName, 'Retry replay-safe API requests after transient failures')
            [CompletionResult]::new('--profile', '--profile', [CompletionResultType]::ParameterName, 'Isolate credentials and unlock state under a named profile')
            [CompletionResult]::new('--config-dir', '--config-dir', [CompletionResultType]::ParameterName, 'Override the base directory used for credentials and local unlock state')
            [CompletionResult]::new('--input-stdin', '--input-stdin', [CompletionResultType]::ParameterName, 'Read an optional audit patch from stdin')
            [CompletionResult]::new('--password-stdin', '--password-stdin', [CompletionResultType]::ParameterName, 'Read the account password from stdin while resolving human selectors')
            [CompletionResult]::new('--yes', '--yes', [CompletionResultType]::ParameterName, 'Confirm permanent deletion without prompting')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit compact JSON instead of human-readable output')
            [CompletionResult]::new('--no-pager', '--no-pager', [CompletionResultType]::ParameterName, 'Disable paging (equivalent to --pager never)')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Suppress automatic paging, progress, and successful mutation acknowledgements')
            [CompletionResult]::new('--non-interactive', '--non-interactive', [CompletionResultType]::ParameterName, 'Never prompt; fail with an actionable validation error when input is missing')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Emit redacted operator telemetry to stderr; repeat for more detail')
            [CompletionResult]::new('--debug', '--debug', [CompletionResultType]::ParameterName, 'Emit maximum redacted diagnostic telemetry to stderr')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Read only from the encrypted local snapshot and never access the network')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'sealtask;notes;help' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List decrypted notes in a project')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted note')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted shared or private note')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit a note''s title and Markdown body in your configured editor')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Patch an encrypted note')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a note')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;notes;help;list' {
            break
        }
        'sealtask;notes;help;get' {
            break
        }
        'sealtask;notes;help;create' {
            break
        }
        'sealtask;notes;help;edit' {
            break
        }
        'sealtask;notes;help;update' {
            break
        }
        'sealtask;notes;help;delete' {
            break
        }
        'sealtask;notes;help;help' {
            break
        }
        'sealtask;help' {
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate a shell completion script without reading configuration or credentials')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Render a manual page for the root command or a nested command path')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show machine-readable CLI capabilities and contract versions')
            [CompletionResult]::new('schema', 'schema', [CompletionResultType]::ParameterValue, 'Describe commands and arguments as human help or versioned JSON')
            [CompletionResult]::new('auth', 'auth', [CompletionResultType]::ParameterValue, 'Authenticate, inspect the session, and manage local unlock state')
            [CompletionResult]::new('me', 'me', [CompletionResultType]::ParameterValue, 'Show the current authenticated user')
            [CompletionResult]::new('pick', 'pick', [CompletionResultType]::ParameterValue, 'Fuzzy-pick an entity while printing only a reusable opaque selector')
            [CompletionResult]::new('projects', 'projects', [CompletionResultType]::ParameterValue, 'List, inspect, select, archive, or restore projects')
            [CompletionResult]::new('tasks', 'tasks', [CompletionResultType]::ParameterValue, 'List, inspect, create, update, move, or delete tasks')
            [CompletionResult]::new('stats', 'stats', [CompletionResultType]::ParameterValue, 'Show current dashboard task counts')
            [CompletionResult]::new('activity', 'activity', [CompletionResultType]::ParameterValue, 'Inspect or continuously follow recent account activity')
            [CompletionResult]::new('browse', 'browse', [CompletionResultType]::ParameterValue, 'Browse cached or live decrypted projects and tasks in a private TTY')
            [CompletionResult]::new('cache', 'cache', [CompletionResultType]::ParameterValue, 'Inspect, verify, or clear the encrypted local read cache')
            [CompletionResult]::new('batch', 'batch', [CompletionResultType]::ParameterValue, 'Validate, execute, and safely resume task mutations from JSON Lines')
            [CompletionResult]::new('doctor', 'doctor', [CompletionResultType]::ParameterValue, 'Diagnose local state, authentication, unlock, and API connectivity')
            [CompletionResult]::new('config', 'config', [CompletionResultType]::ParameterValue, 'Inspect resolved operator configuration')
            [CompletionResult]::new('profile', 'profile', [CompletionResultType]::ParameterValue, 'List profiles or change the persisted default profile')
            [CompletionResult]::new('inspect', 'inspect', [CompletionResultType]::ParameterValue, 'inspect')
            [CompletionResult]::new('comments', 'comments', [CompletionResultType]::ParameterValue, 'List, create, update, or delete task comments')
            [CompletionResult]::new('notes', 'notes', [CompletionResultType]::ParameterValue, 'List, inspect, create, update, or delete encrypted notes')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'sealtask;help;completion' {
            break
        }
        'sealtask;help;man' {
            break
        }
        'sealtask;help;info' {
            break
        }
        'sealtask;help;schema' {
            break
        }
        'sealtask;help;auth' {
            [CompletionResult]::new('login', 'login', [CompletionResultType]::ParameterValue, 'Sign in with an email and password, optionally completing MFA')
            [CompletionResult]::new('unlock', 'unlock', [CompletionResultType]::ParameterValue, 'Unlock workspace data in memory for a bounded session')
            [CompletionResult]::new('lock', 'lock', [CompletionResultType]::ParameterValue, 'Lock workspace data and stop the in-memory unlock session')
            [CompletionResult]::new('keychain', 'keychain', [CompletionResultType]::ParameterValue, 'Store or clear this profile''s saved unlock key')
            [CompletionResult]::new('logout', 'logout', [CompletionResultType]::ParameterValue, 'Revoke the remote session and clear this profile''s local credentials')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Inspect sign-in, token expiry, workspace-data, and saved-key state')
            break
        }
        'sealtask;help;auth;login' {
            break
        }
        'sealtask;help;auth;unlock' {
            break
        }
        'sealtask;help;auth;lock' {
            break
        }
        'sealtask;help;auth;keychain' {
            [CompletionResult]::new('store', 'store', [CompletionResultType]::ParameterValue, 'Save this profile''s unlock key in the platform keychain')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove this profile''s saved unlock key')
            break
        }
        'sealtask;help;auth;keychain;store' {
            break
        }
        'sealtask;help;auth;keychain;clear' {
            break
        }
        'sealtask;help;auth;logout' {
            break
        }
        'sealtask;help;auth;status' {
            break
        }
        'sealtask;help;me' {
            break
        }
        'sealtask;help;pick' {
            [CompletionResult]::new('project', 'project', [CompletionResultType]::ParameterValue, 'Pick an accessible project')
            [CompletionResult]::new('task', 'task', [CompletionResultType]::ParameterValue, 'Pick a task in the selected/current project')
            break
        }
        'sealtask;help;pick;project' {
            break
        }
        'sealtask;help;pick;task' {
            break
        }
        'sealtask;help;projects' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List accessible projects')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted project')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a project and make it read-only')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived project')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Save a project as the current project for this profile')
            [CompletionResult]::new('current', 'current', [CompletionResultType]::ParameterValue, 'Show the saved current project without accessing the network')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Clear the saved current project')
            [CompletionResult]::new('sections', 'sections', [CompletionResultType]::ParameterValue, 'Discover sections in a project')
            [CompletionResult]::new('audit', 'audit', [CompletionResultType]::ParameterValue, 'Show a bounded page of safe project audit metadata')
            break
        }
        'sealtask;help;projects;list' {
            break
        }
        'sealtask;help;projects;get' {
            break
        }
        'sealtask;help;projects;archive' {
            break
        }
        'sealtask;help;projects;unarchive' {
            break
        }
        'sealtask;help;projects;use' {
            break
        }
        'sealtask;help;projects;current' {
            break
        }
        'sealtask;help;projects;clear' {
            break
        }
        'sealtask;help;projects;sections' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List normalized project sections and their IDs')
            break
        }
        'sealtask;help;projects;sections;list' {
            break
        }
        'sealtask;help;projects;audit' {
            break
        }
        'sealtask;help;tasks' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List tasks in the selected/current project, or assigned tasks when none is selected')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted task, including comments and attachment metadata')
            [CompletionResult]::new('watch', 'watch', [CompletionResultType]::ParameterValue, 'Follow authoritative task changes in one project until interrupted')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted task')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit a task''s title and Markdown body in your configured editor')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Patch an encrypted task; omitted fields remain unchanged')
            [CompletionResult]::new('move', 'move', [CompletionResultType]::ParameterValue, 'Move a task to a section or relative position')
            [CompletionResult]::new('complete', 'complete', [CompletionResultType]::ParameterValue, 'Move a task to the final section')
            [CompletionResult]::new('reopen', 'reopen', [CompletionResultType]::ParameterValue, 'Move a task to the first section')
            [CompletionResult]::new('archive', 'archive', [CompletionResultType]::ParameterValue, 'Archive a task')
            [CompletionResult]::new('unarchive', 'unarchive', [CompletionResultType]::ParameterValue, 'Restore an archived task')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a task')
            [CompletionResult]::new('attachments', 'attachments', [CompletionResultType]::ParameterValue, 'Upload, delete, read, or download encrypted task attachments')
            break
        }
        'sealtask;help;tasks;list' {
            break
        }
        'sealtask;help;tasks;get' {
            break
        }
        'sealtask;help;tasks;watch' {
            break
        }
        'sealtask;help;tasks;create' {
            break
        }
        'sealtask;help;tasks;edit' {
            break
        }
        'sealtask;help;tasks;update' {
            break
        }
        'sealtask;help;tasks;move' {
            break
        }
        'sealtask;help;tasks;complete' {
            break
        }
        'sealtask;help;tasks;reopen' {
            break
        }
        'sealtask;help;tasks;archive' {
            break
        }
        'sealtask;help;tasks;unarchive' {
            break
        }
        'sealtask;help;tasks;delete' {
            break
        }
        'sealtask;help;tasks;attachments' {
            [CompletionResult]::new('upload', 'upload', [CompletionResultType]::ParameterValue, 'Encrypt and upload a local regular file')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Remove an attachment reference and its encrypted object')
            [CompletionResult]::new('read', 'read', [CompletionResultType]::ParameterValue, 'Decrypt a text or DOCX attachment and print readable text')
            [CompletionResult]::new('download', 'download', [CompletionResultType]::ParameterValue, 'Decrypt an attachment and save it beneath the current directory')
            break
        }
        'sealtask;help;tasks;attachments;upload' {
            break
        }
        'sealtask;help;tasks;attachments;delete' {
            break
        }
        'sealtask;help;tasks;attachments;read' {
            break
        }
        'sealtask;help;tasks;attachments;download' {
            break
        }
        'sealtask;help;stats' {
            break
        }
        'sealtask;help;activity' {
            [CompletionResult]::new('follow', 'follow', [CompletionResultType]::ParameterValue, 'Follow new activity using bounded cursor catch-up polling')
            break
        }
        'sealtask;help;activity;follow' {
            break
        }
        'sealtask;help;browse' {
            break
        }
        'sealtask;help;cache' {
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Show cache presence, mode, size, and modification time without decrypting content')
            [CompletionResult]::new('verify', 'verify', [CompletionResultType]::ParameterValue, 'Authenticate, decrypt, and validate the complete local cache')
            [CompletionResult]::new('clear', 'clear', [CompletionResultType]::ParameterValue, 'Remove the encrypted local cache for the active profile')
            break
        }
        'sealtask;help;cache;status' {
            break
        }
        'sealtask;help;cache;verify' {
            break
        }
        'sealtask;help;cache;clear' {
            break
        }
        'sealtask;help;batch' {
            [CompletionResult]::new('run', 'run', [CompletionResultType]::ParameterValue, 'Run a strict versioned JSONL task-mutation batch')
            break
        }
        'sealtask;help;batch;run' {
            break
        }
        'sealtask;help;doctor' {
            break
        }
        'sealtask;help;config' {
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show safe configuration values and where they came from')
            break
        }
        'sealtask;help;config;show' {
            break
        }
        'sealtask;help;profile' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List known local profiles and mark the active one')
            [CompletionResult]::new('use', 'use', [CompletionResultType]::ParameterValue, 'Persist the default profile for future commands')
            break
        }
        'sealtask;help;profile;list' {
            break
        }
        'sealtask;help;profile;use' {
            break
        }
        'sealtask;help;inspect' {
            break
        }
        'sealtask;help;comments' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List decrypted comments on a task')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted task comment')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Replace an encrypted task comment body')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a task comment')
            break
        }
        'sealtask;help;comments;list' {
            break
        }
        'sealtask;help;comments;create' {
            break
        }
        'sealtask;help;comments;update' {
            break
        }
        'sealtask;help;comments;delete' {
            break
        }
        'sealtask;help;notes' {
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List decrypted notes in a project')
            [CompletionResult]::new('get', 'get', [CompletionResultType]::ParameterValue, 'Show one decrypted note')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an encrypted shared or private note')
            [CompletionResult]::new('edit', 'edit', [CompletionResultType]::ParameterValue, 'Edit a note''s title and Markdown body in your configured editor')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Patch an encrypted note')
            [CompletionResult]::new('delete', 'delete', [CompletionResultType]::ParameterValue, 'Permanently delete a note')
            break
        }
        'sealtask;help;notes;list' {
            break
        }
        'sealtask;help;notes;get' {
            break
        }
        'sealtask;help;notes;create' {
            break
        }
        'sealtask;help;notes;edit' {
            break
        }
        'sealtask;help;notes;update' {
            break
        }
        'sealtask;help;notes;delete' {
            break
        }
        'sealtask;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
