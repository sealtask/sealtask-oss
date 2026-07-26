use chrono::{DateTime, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use sealtask_client_core::{PublicError, PublicResult};
use std::str::FromStr;

const DEFAULT_UNLOCK_TTL_SECONDS: u64 = 8 * 60 * 60;

pub(crate) fn parse_priority(value: &str) -> Result<i8, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "low" | "p4" => Ok(1),
        "3" | "medium" | "p3" => Ok(3),
        "5" | "high" | "p2" => Ok(5),
        "8" | "urgent" | "p1" => Ok(8),
        _ => Err("priority must be low/p4/1, medium/p3/3, high/p2/5, or urgent/p1/8".to_string()),
    }
}

pub(crate) fn resolve_unlock_ttl(ttl: Option<&str>, ttl_seconds: Option<u64>) -> PublicResult<u64> {
    match (ttl, ttl_seconds) {
        (Some(value), None) => parse_duration_seconds(value),
        (None, Some(value)) => validate_duration_seconds(value),
        (None, None) => Ok(DEFAULT_UNLOCK_TTL_SECONDS),
        (Some(_), Some(_)) => Err(PublicError::validation(
            "--ttl cannot be used with --ttl-seconds",
        )),
    }
}

pub(crate) fn parse_duration_seconds(value: &str) -> PublicResult<u64> {
    let value = value.trim();
    if value.is_empty() {
        return Err(PublicError::validation("duration must not be empty"));
    }

    let bytes = value.as_bytes();
    let mut index = 0;
    let mut total = 0_u64;
    while index < bytes.len() {
        let digits_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if digits_start == index || index == bytes.len() {
            return Err(PublicError::validation(format!(
                "invalid duration '{value}'; use components such as 30m, 8h, 1h30m, or 2d"
            )));
        }
        let amount = value[digits_start..index].parse::<u64>().map_err(|_| {
            PublicError::validation(format!("duration component is too large in '{value}'"))
        })?;
        let multiplier = match bytes[index].to_ascii_lowercase() {
            b's' => 1,
            b'm' => 60,
            b'h' => 60 * 60,
            b'd' => 24 * 60 * 60,
            b'w' => 7 * 24 * 60 * 60,
            _ => {
                return Err(PublicError::validation(format!(
                    "invalid duration unit in '{value}'; use s, m, h, d, or w"
                )));
            }
        };
        index += 1;
        total = total
            .checked_add(amount.checked_mul(multiplier).ok_or_else(|| {
                PublicError::validation(format!("duration is too large: '{value}'"))
            })?)
            .ok_or_else(|| PublicError::validation(format!("duration is too large: '{value}'")))?;
    }
    validate_duration_seconds(total)
}

fn validate_duration_seconds(value: u64) -> PublicResult<u64> {
    if value == 0 {
        return Err(PublicError::validation(
            "unlock TTL must be greater than zero",
        ));
    }
    Ok(value)
}

pub(crate) fn parse_due_input(
    value: &str,
    project_timezone: &str,
    now: DateTime<Utc>,
) -> PublicResult<DateTime<Utc>> {
    if let Ok(value) = DateTime::parse_from_rfc3339(value) {
        return Ok(value.with_timezone(&Utc));
    }

    let timezone = Tz::from_str(project_timezone).map_err(|_| {
        PublicError::validation(format!(
            "project timezone '{project_timezone}' is invalid; use --due-at with an RFC 3339 timestamp"
        ))
    })?;
    let trimmed = value.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let local_now = now.with_timezone(&timezone);

    let (date, time, explicit_time) = if let Some(rest) = lowered.strip_prefix("today") {
        (
            local_now.date_naive(),
            parse_optional_time(rest, trimmed)?,
            !rest.trim().is_empty(),
        )
    } else if let Some(rest) = lowered.strip_prefix("tomorrow") {
        (
            local_now
                .date_naive()
                .checked_add_signed(Duration::days(1))
                .ok_or_else(|| {
                    PublicError::validation("tomorrow is outside the supported range")
                })?,
            parse_optional_time(rest, trimmed)?,
            !rest.trim().is_empty(),
        )
    } else if let Ok(date_time) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M") {
        (date_time.date(), date_time.time(), true)
    } else if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        (
            date,
            NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is valid"),
            false,
        )
    } else {
        return Err(PublicError::validation(format!(
            "invalid due date '{trimmed}'; use RFC 3339, YYYY-MM-DD, YYYY-MM-DDTHH:MM, today, or tomorrow"
        )));
    };

    if explicit_time {
        return local_datetime_to_utc(timezone, date.and_time(time), trimmed);
    }

    for minutes in 0..(24 * 60) {
        let candidate = date
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid")
            .checked_add_signed(Duration::minutes(minutes))
            .ok_or_else(|| PublicError::validation("due date is outside the supported range"))?;
        match timezone.from_local_datetime(&candidate) {
            LocalResult::Single(value) => return Ok(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(earlier, _) => return Ok(earlier.with_timezone(&Utc)),
            LocalResult::None => {}
        }
    }
    Err(PublicError::validation(format!(
        "calendar date {date} does not exist in timezone {project_timezone}"
    )))
}

fn parse_optional_time(rest: &str, original: &str) -> PublicResult<NaiveTime> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is valid"));
    }
    NaiveTime::parse_from_str(rest, "%H:%M").map_err(|_| {
        PublicError::validation(format!(
            "invalid relative due date '{original}'; use 'today HH:MM' or 'tomorrow HH:MM'"
        ))
    })
}

fn local_datetime_to_utc(
    timezone: Tz,
    date_time: NaiveDateTime,
    original: &str,
) -> PublicResult<DateTime<Utc>> {
    match timezone.from_local_datetime(&date_time) {
        LocalResult::Single(value) => Ok(value.with_timezone(&Utc)),
        LocalResult::Ambiguous(_, _) => Err(PublicError::validation(format!(
            "local time '{original}' is ambiguous in timezone {timezone}; use RFC 3339 with an explicit offset"
        ))),
        LocalResult::None => Err(PublicError::validation(format!(
            "local time '{original}' does not exist in timezone {timezone}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn priority_aliases_map_to_wire_values() {
        for (input, expected) in [
            ("low", 1),
            ("P4", 1),
            ("medium", 3),
            ("p3", 3),
            ("high", 5),
            ("p2", 5),
            ("urgent", 8),
            ("P1", 8),
        ] {
            assert_eq!(parse_priority(input), Ok(expected));
        }
    }

    #[test]
    fn compound_durations_are_checked() {
        assert_eq!(parse_duration_seconds("1h30m").expect("duration"), 5_400);
        assert_eq!(parse_duration_seconds("2d").expect("duration"), 172_800);
        assert!(parse_duration_seconds("0s").is_err());
        assert!(parse_duration_seconds("1.5h").is_err());
        assert!(parse_duration_seconds("18446744073709551615w").is_err());
    }

    #[test]
    fn relative_dates_use_the_project_timezone() {
        let now = Utc
            .with_ymd_and_hms(2026, 7, 26, 22, 30, 0)
            .single()
            .expect("instant");
        let due = parse_due_input("tomorrow", "Europe/Prague", now).expect("due");
        assert_eq!(due.to_rfc3339(), "2026-07-27T22:00:00+00:00");
    }

    #[test]
    fn explicit_nonexistent_local_time_is_rejected() {
        let now = Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("instant");
        let error = parse_due_input("2026-03-29T02:30", "Europe/Prague", now).expect_err("DST gap");
        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn ambiguous_local_time_requires_an_explicit_offset() {
        let now = Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .expect("instant");
        let error =
            parse_due_input("2026-10-25T02:30", "Europe/Prague", now).expect_err("DST overlap");
        let message = error.to_string();
        assert!(message.contains("ambiguous"));
        assert!(message.contains("RFC 3339"));
    }

    #[test]
    fn rfc3339_preserves_the_instant() {
        let now = Utc::now();
        let parsed = parse_due_input("2026-07-26T12:30:00+02:00", "Invalid/Zone", now)
            .expect("RFC 3339 bypasses timezone");
        assert_eq!(parsed.to_rfc3339(), "2026-07-26T10:30:00+00:00");
    }
}
