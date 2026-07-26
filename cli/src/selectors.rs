use sealtask_client_api::SectionSnapshotPayload;
use sealtask_client_core::{PublicError, PublicResult};
use sealtask_client_runtime::AgentWorkListSummary;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

const MINIMUM_ID_PREFIX_LENGTH: usize = 8;
const MAXIMUM_SELECTOR_BYTES: usize = 1_024;

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct EntitySelector(String);

impl EntitySelector {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for EntitySelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EntitySelector(<redacted>)")
    }
}

impl FromStr for EntitySelector {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err("selector must not be empty".to_string());
        }
        if value.len() > MAXIMUM_SELECTOR_BYTES {
            return Err(format!(
                "selector must be at most {MAXIMUM_SELECTOR_BYTES} bytes"
            ));
        }
        if value.chars().any(char::is_control) {
            return Err("selector must not contain control characters".to_string());
        }
        if matches!(value, "id:" | "name:") {
            return Err("selector prefix must be followed by a value".to_string());
        }
        Ok(Self(value.to_string()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EntityCandidate {
    pub(crate) id: Uuid,
    pub(crate) name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedEntity {
    pub(crate) id: Uuid,
    pub(crate) name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectSection {
    pub(crate) id: Uuid,
    pub(crate) project_id: Uuid,
    pub(crate) name: Option<String>,
    pub(crate) position: i64,
    pub(crate) wip_limit: Option<i64>,
    pub(crate) auto_archive_enabled: bool,
    pub(crate) auto_archive_after_days: Option<i32>,
}

pub(crate) fn resolve_entity(
    entity_kind: &str,
    scope: &str,
    selector: &EntitySelector,
    mut candidates: Vec<EntityCandidate>,
    list_command: &str,
) -> PublicResult<ResolvedEntity> {
    let raw = selector.as_str();
    if let Ok(id) = Uuid::parse_str(raw) {
        return Ok(ResolvedEntity { id, name: None });
    }

    candidates.sort_by(|left, right| {
        normalized_name(left.name.as_deref().unwrap_or(""))
            .cmp(&normalized_name(right.name.as_deref().unwrap_or("")))
            .then_with(|| left.id.cmp(&right.id))
    });

    if let Some(name) = raw.strip_prefix("name:") {
        return resolve_name(entity_kind, scope, name, &candidates, list_command);
    }
    if let Some(prefix) = raw.strip_prefix("id:") {
        return resolve_id_prefix(entity_kind, scope, prefix, &candidates, list_command);
    }

    let exact_name_matches = candidates
        .iter()
        .filter(|candidate| candidate.name.as_deref() == Some(raw))
        .cloned()
        .collect::<Vec<_>>();
    if !exact_name_matches.is_empty() {
        return one_match_or_ambiguity(entity_kind, scope, raw, exact_name_matches);
    }

    let normalized = normalized_name(raw);
    let normalized_matches = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .name
                .as_deref()
                .is_some_and(|name| normalized_name(name) == normalized)
        })
        .cloned()
        .collect::<Vec<_>>();
    if !normalized_matches.is_empty() {
        return one_match_or_ambiguity(entity_kind, scope, raw, normalized_matches);
    }

    if looks_like_id_prefix(raw) {
        return resolve_id_prefix(entity_kind, scope, raw, &candidates, list_command);
    }

    Err(not_found(entity_kind, scope, raw, list_command, "name"))
}

fn resolve_name(
    entity_kind: &str,
    scope: &str,
    name: &str,
    candidates: &[EntityCandidate],
    list_command: &str,
) -> PublicResult<ResolvedEntity> {
    if name.trim().is_empty() {
        return Err(PublicError::validation("name: selector must not be empty"));
    }
    let exact = candidates
        .iter()
        .filter(|candidate| candidate.name.as_deref() == Some(name))
        .cloned()
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return one_match_or_ambiguity(entity_kind, scope, name, exact);
    }

    let normalized = normalized_name(name);
    let matches = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .name
                .as_deref()
                .is_some_and(|candidate_name| normalized_name(candidate_name) == normalized)
        })
        .cloned()
        .collect::<Vec<_>>();
    if matches.is_empty() {
        Err(not_found(entity_kind, scope, name, list_command, "name"))
    } else {
        one_match_or_ambiguity(entity_kind, scope, name, matches)
    }
}

fn resolve_id_prefix(
    entity_kind: &str,
    scope: &str,
    prefix: &str,
    candidates: &[EntityCandidate],
    list_command: &str,
) -> PublicResult<ResolvedEntity> {
    let normalized = normalize_id_prefix(prefix)?;
    let matches = candidates
        .iter()
        .filter(|candidate| simple_id(candidate.id).starts_with(&normalized))
        .cloned()
        .collect::<Vec<_>>();
    if matches.is_empty() {
        Err(not_found(
            entity_kind,
            scope,
            prefix,
            list_command,
            "ID prefix",
        ))
    } else {
        one_match_or_ambiguity(entity_kind, scope, prefix, matches)
    }
}

fn one_match_or_ambiguity(
    entity_kind: &str,
    scope: &str,
    selector: &str,
    matches: Vec<EntityCandidate>,
) -> PublicResult<ResolvedEntity> {
    if let [candidate] = matches.as_slice() {
        return Ok(ResolvedEntity {
            id: candidate.id,
            name: candidate.name.clone(),
        });
    }

    let choices = matches
        .iter()
        .map(|candidate| {
            format!(
                "{} ({})",
                candidate.name.as_deref().unwrap_or("<unnamed>"),
                candidate.id
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    Err(PublicError::validation(format!(
        "ambiguous {entity_kind} selector '{selector}' in {scope}; matches: {choices}; retry with id:<at-least-8-hex-digits>"
    )))
}

fn not_found(
    entity_kind: &str,
    scope: &str,
    selector: &str,
    list_command: &str,
    selector_kind: &str,
) -> PublicError {
    PublicError::validation(format!(
        "no {entity_kind} matching {selector_kind} '{selector}' was found in {scope}; run '{list_command}' to discover valid targets"
    ))
}

fn normalize_id_prefix(value: &str) -> PublicResult<String> {
    let normalized = value
        .trim()
        .chars()
        .filter(|character| *character != '-')
        .collect::<String>()
        .to_ascii_lowercase();
    if normalized.len() < MINIMUM_ID_PREFIX_LENGTH {
        return Err(PublicError::validation(format!(
            "ID prefixes must contain at least {MINIMUM_ID_PREFIX_LENGTH} hexadecimal characters"
        )));
    }
    if !normalized
        .chars()
        .all(|character| character.is_ascii_hexdigit())
    {
        return Err(PublicError::validation(
            "ID prefixes may contain only hexadecimal characters and hyphens",
        ));
    }
    if normalized.len() > 32 {
        return Err(PublicError::validation(
            "ID prefixes must not exceed a UUID's 32 hexadecimal characters",
        ));
    }
    Ok(normalized)
}

fn looks_like_id_prefix(value: &str) -> bool {
    let compact = value.chars().filter(|character| *character != '-');
    let mut length = 0;
    for character in compact {
        if !character.is_ascii_hexdigit() {
            return false;
        }
        length += 1;
    }
    length >= MINIMUM_ID_PREFIX_LENGTH
}

fn simple_id(id: Uuid) -> String {
    id.simple().to_string()
}

fn normalized_name(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect()
}

pub(crate) fn project_sections(
    project: &AgentWorkListSummary,
) -> PublicResult<Vec<ProjectSection>> {
    let payload_sections = parse_payload_sections(project)?;
    let mut by_id = payload_sections
        .into_iter()
        .map(|section| (section.id, section))
        .collect::<HashMap<_, _>>();
    let mut snapshots = project.section_snapshots.clone();
    snapshots.sort_by_key(|snapshot| (snapshot.position, snapshot.id));

    let mut seen = HashSet::new();
    let mut sections = Vec::with_capacity(snapshots.len() + by_id.len());
    for snapshot in snapshots {
        if !seen.insert(snapshot.id) {
            return Err(PublicError::validation(format!(
                "project {} contains duplicate section snapshot {}",
                project.id, snapshot.id
            )));
        }
        let payload = by_id.remove(&snapshot.id);
        sections.push(section_from_snapshot(project.id, snapshot, payload));
    }

    let next_position = sections
        .last()
        .map_or(0, |section| section.position.saturating_add(1));
    let mut payload_only = by_id.into_values().collect::<Vec<_>>();
    payload_only.sort_by_key(|section| (section.position, section.id));
    for (offset, mut section) in payload_only.into_iter().enumerate() {
        section.position = next_position.saturating_add(offset as i64);
        sections.push(section);
    }
    Ok(sections)
}

fn section_from_snapshot(
    project_id: Uuid,
    snapshot: SectionSnapshotPayload,
    payload: Option<ProjectSection>,
) -> ProjectSection {
    ProjectSection {
        id: snapshot.id,
        project_id,
        name: payload.as_ref().and_then(|section| section.name.clone()),
        position: snapshot.position,
        wip_limit: payload.and_then(|section| section.wip_limit),
        auto_archive_enabled: snapshot.auto_archive_enabled,
        auto_archive_after_days: snapshot.auto_archive_after_days,
    }
}

fn parse_payload_sections(project: &AgentWorkListSummary) -> PublicResult<Vec<ProjectSection>> {
    let Some(sections) = project
        .payload
        .as_ref()
        .and_then(|payload| payload.get("body"))
        .and_then(|body| body.get("sections"))
    else {
        return Ok(Vec::new());
    };
    let sections = sections.as_array().ok_or_else(|| {
        PublicError::validation(format!(
            "project {} has malformed section metadata; expected an array",
            project.id
        ))
    })?;

    let mut seen = HashSet::new();
    sections
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let object = value.as_object().ok_or_else(|| {
                PublicError::validation(format!(
                    "project {} section {} is not an object",
                    project.id,
                    index + 1
                ))
            })?;
            let id = object
                .get("id")
                .and_then(project_payload_uuid)
                .ok_or_else(|| {
                    PublicError::validation(format!(
                        "project {} section {} is missing a valid UUID id",
                        project.id,
                        index + 1
                    ))
                })?;
            if !seen.insert(id) {
                return Err(PublicError::validation(format!(
                    "project {} contains duplicate section {}",
                    project.id, id
                )));
            }
            let name = object
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned);
            let wip_limit = object
                .get("wip_limit")
                .or_else(|| object.get("wipLimit"))
                .and_then(serde_json::Value::as_i64);
            Ok(ProjectSection {
                id,
                project_id: project.id,
                name,
                position: index as i64,
                wip_limit,
                auto_archive_enabled: false,
                auto_archive_after_days: None,
            })
        })
        .collect()
}

fn project_payload_uuid(value: &serde_json::Value) -> Option<Uuid> {
    if let Some(value) = value.as_str() {
        return Uuid::parse_str(value).ok();
    }

    let bytes = value
        .as_array()?
        .iter()
        .map(|byte| byte.as_u64().and_then(|byte| u8::try_from(byte).ok()))
        .collect::<Option<Vec<_>>>()?;
    Uuid::from_slice(&bytes).ok()
}

pub(crate) fn section_candidates(sections: &[ProjectSection]) -> Vec<EntityCandidate> {
    sections
        .iter()
        .map(|section| EntityCandidate {
            id: section.id,
            name: section.name.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sealtask_client_runtime::AgentMembership;

    fn selector(value: &str) -> EntitySelector {
        value.parse().expect("selector")
    }

    fn candidate(id: &str, name: &str) -> EntityCandidate {
        EntityCandidate {
            id: Uuid::parse_str(id).expect("UUID"),
            name: Some(name.to_string()),
        }
    }

    fn project_with_sections(
        sections: serde_json::Value,
        snapshots: Vec<SectionSnapshotPayload>,
    ) -> AgentWorkListSummary {
        let now = Utc::now();
        AgentWorkListSummary {
            id: Uuid::from_u128(100),
            owner_user_id: Uuid::from_u128(101),
            workspace_id: Uuid::from_u128(102),
            timezone: "UTC".to_string(),
            section_snapshots: snapshots,
            created_at: now,
            updated_at: now,
            archived_at: None,
            membership: AgentMembership {
                id: Uuid::from_u128(103),
                user_id: Uuid::from_u128(104),
                user_email: "operator@example.com".to_string(),
                user_name: "Operator".to_string(),
                user_avatar_color: "#000000".to_string(),
                role: "owner".to_string(),
                status: "active".to_string(),
                expires_at: None,
                joined_at: now,
            },
            title: Some("Project".to_string()),
            description: None,
            payload: Some(serde_json::json!({
                "body": {
                    "sections": sections,
                },
            })),
            read_error: None,
        }
    }

    #[test]
    fn selector_debug_never_exposes_plaintext() {
        let value = selector("customer-secret-project");
        let debug = format!("{value:?}");
        assert_eq!(debug, "EntitySelector(<redacted>)");
        assert!(!debug.contains(value.as_str()));
    }

    #[test]
    fn names_use_exact_then_unicode_normalized_matching() {
        let candidates = vec![
            candidate("01900000-0000-7000-8000-000000000001", "Résumé"),
            candidate("01900000-0000-7000-8000-000000000002", "Release"),
        ];
        let resolved = resolve_entity(
            "project",
            "accessible projects",
            &selector("re\u{301}sume\u{301}"),
            candidates,
            "sealtask projects list",
        )
        .expect("normalized match");
        assert_eq!(
            resolved.id,
            Uuid::parse_str("01900000-0000-7000-8000-000000000001").expect("UUID")
        );
    }

    #[test]
    fn unique_eight_digit_prefix_and_explicit_prefix_resolve() {
        let candidates = vec![
            candidate("01900000-0000-7000-8000-000000000001", "One"),
            candidate("02900000-0000-7000-8000-000000000002", "Two"),
        ];
        for value in ["01900000", "id:01900000"] {
            let resolved = resolve_entity(
                "task",
                "project",
                &selector(value),
                candidates.clone(),
                "sealtask tasks list",
            )
            .expect("prefix match");
            assert_eq!(
                resolved.id,
                Uuid::parse_str("01900000-0000-7000-8000-000000000001").expect("UUID")
            );
        }
    }

    #[test]
    fn explicit_id_prefix_cannot_be_shadowed_by_an_entity_name() {
        let id_target = candidate("01900000-0000-7000-8000-000000000001", "Target");
        let name_target = candidate("02900000-0000-7000-8000-000000000002", "01900000");
        let candidates = vec![id_target.clone(), name_target.clone()];

        let bare = resolve_entity(
            "task",
            "project",
            &selector("01900000"),
            candidates.clone(),
            "sealtask tasks list",
        )
        .expect("bare selector follows exact-name precedence");
        assert_eq!(bare.id, name_target.id);

        let explicit = resolve_entity(
            "task",
            "project",
            &selector("id:01900000"),
            candidates,
            "sealtask tasks list",
        )
        .expect("explicit ID prefix");
        assert_eq!(explicit.id, id_target.id);
    }

    #[test]
    fn ambiguous_errors_are_deterministic() {
        let first = candidate("01900000-0000-7000-8000-000000000002", "Same");
        let second = candidate("01900000-0000-7000-8000-000000000001", "same");
        let error_a = resolve_entity(
            "task",
            "project",
            &selector("name:SAME"),
            vec![first.clone(), second.clone()],
            "sealtask tasks list",
        )
        .expect_err("ambiguous");
        let error_b = resolve_entity(
            "task",
            "project",
            &selector("name:SAME"),
            vec![second, first],
            "sealtask tasks list",
        )
        .expect_err("ambiguous");
        assert_eq!(error_a.to_string(), error_b.to_string());
    }

    #[test]
    fn short_prefixes_are_rejected() {
        let error = resolve_entity(
            "task",
            "project",
            &selector("id:0190"),
            Vec::new(),
            "sealtask tasks list",
        )
        .expect_err("short prefix");
        assert!(error.to_string().contains("at least 8"));
    }

    #[test]
    fn project_section_ids_accept_text_and_canonical_cbor_bytes() {
        let id = Uuid::parse_str("01900000-0000-7000-8000-000000000001").expect("UUID");
        assert_eq!(
            project_payload_uuid(&serde_json::Value::String(id.to_string())),
            Some(id)
        );
        assert_eq!(
            project_payload_uuid(&serde_json::Value::Array(
                id.as_bytes()
                    .iter()
                    .copied()
                    .map(serde_json::Value::from)
                    .collect(),
            )),
            Some(id)
        );
        assert_eq!(project_payload_uuid(&serde_json::json!([256, 0, 1])), None);
    }

    #[test]
    fn payload_only_sections_preserve_encrypted_project_order() {
        let first = Uuid::from_u128(110);
        let second = Uuid::from_u128(111);
        let project = project_with_sections(
            serde_json::json!([
                {"id": first, "name": "Zulu"},
                {"id": second, "name": "Alpha"},
            ]),
            Vec::new(),
        );

        let sections = project_sections(&project).expect("project sections");
        assert_eq!(
            sections
                .iter()
                .map(|section| section.id)
                .collect::<Vec<_>>(),
            [first, second]
        );
    }
}
