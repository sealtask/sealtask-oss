use crate::output::{CliError, terminal_line, write_stderr_line};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TelemetryLevel {
    Off,
    Verbose,
    Trace,
}

impl TelemetryLevel {
    pub(crate) fn from_flags(verbosity: u8, debug: bool) -> Self {
        if debug || verbosity >= 2 {
            Self::Trace
        } else if verbosity == 1 {
            Self::Verbose
        } else {
            Self::Off
        }
    }

    pub(crate) const fn enabled(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub(crate) const fn traces(self) -> bool {
        matches!(self, Self::Trace)
    }
}

pub(crate) struct Telemetry {
    level: TelemetryLevel,
    invocation_id: Uuid,
    command: &'static str,
    started_at: Instant,
}

pub(crate) struct TelemetryConfig<'a> {
    pub(crate) api_url: &'a str,
    pub(crate) profile_is_default: bool,
    pub(crate) profile_source: &'a str,
    pub(crate) config_dir_source: &'a str,
    pub(crate) timeouts: (Duration, Duration, Duration),
    pub(crate) retry_limit: u8,
}

impl Telemetry {
    pub(crate) fn start(
        level: TelemetryLevel,
        invocation_id: Uuid,
        command: &'static str,
        config: TelemetryConfig<'_>,
    ) -> Self {
        let telemetry = Self {
            level,
            invocation_id,
            command,
            started_at: Instant::now(),
        };
        if level.enabled() {
            telemetry.emit(format_args!(
                "event=start invocation_id={} command={} profile_kind={} profile_source={}",
                telemetry.invocation_id,
                command,
                if config.profile_is_default {
                    "default"
                } else {
                    "named"
                },
                config.profile_source
            ));
        }
        if level.traces() {
            let (connect, read, request) = config.timeouts;
            telemetry.emit(format_args!(
                "event=config invocation_id={} api_origin={} config_dir_source={} connect_timeout_ms={} read_timeout_ms={} request_timeout_ms={} retry_limit={}",
                telemetry.invocation_id,
                safe_api_origin(config.api_url),
                config.config_dir_source,
                connect.as_millis(),
                read.as_millis(),
                request.as_millis(),
                config.retry_limit,
            ));
        }
        telemetry
    }

    pub(crate) fn finish(&self, result: &Result<(), CliError>) {
        if !self.level.enabled() {
            return;
        }
        let elapsed_ms = self.started_at.elapsed().as_millis();
        match result {
            Ok(()) => self.emit(format_args!(
                "event=finish invocation_id={} command={} status=ok elapsed_ms={elapsed_ms}",
                self.invocation_id, self.command
            )),
            Err(error) => self.emit(format_args!(
                "event=finish invocation_id={} command={} status=error code={} elapsed_ms={elapsed_ms}",
                self.invocation_id,
                self.command,
                error.code()
            )),
        }
    }

    fn emit(&self, message: std::fmt::Arguments<'_>) {
        let _ = write_stderr_line(format_args!("[sealtask] {message}"));
    }
}

fn safe_api_origin(api_url: &str) -> String {
    let Ok(url) = reqwest::Url::parse(api_url) else {
        return "<invalid>".to_string();
    };
    let Some(host) = url.host_str() else {
        return "<invalid>".to_string();
    };
    let mut origin = format!("{}://{host}", url.scheme());
    if let Some(port) = url.port() {
        origin.push(':');
        origin.push_str(&port.to_string());
    }
    terminal_line(&origin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_origin_drops_credentials_paths_queries_and_fragments() {
        assert_eq!(
            safe_api_origin("https://user:secret@example.com:8443/private?token=secret#value"),
            "https://example.com:8443"
        );
        assert_eq!(safe_api_origin("not a URL"), "<invalid>");
    }

    #[test]
    fn telemetry_flags_have_bounded_levels() {
        assert_eq!(TelemetryLevel::from_flags(0, false), TelemetryLevel::Off);
        assert_eq!(
            TelemetryLevel::from_flags(1, false),
            TelemetryLevel::Verbose
        );
        assert_eq!(TelemetryLevel::from_flags(2, false), TelemetryLevel::Trace);
        assert_eq!(
            TelemetryLevel::from_flags(u8::MAX, false),
            TelemetryLevel::Trace
        );
        assert_eq!(TelemetryLevel::from_flags(0, true), TelemetryLevel::Trace);
    }
}
