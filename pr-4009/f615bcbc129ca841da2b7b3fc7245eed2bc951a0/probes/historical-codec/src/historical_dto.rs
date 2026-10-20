use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::server::runtime_ws::PROTOCOL_VERSION;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HelloData {
    pub protocol_version: u16,
    pub runtime_epoch: String,
    pub prebind_instance_id: String,
    pub runtime_instance_id: String,
    pub namespace: String,
    pub owner: Owner,
    pub local_owner: bool,
    pub health: AggregateHealth,
    pub profiles: Vec<ProfileHello>,
    pub status_freshness: StatusFreshness,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotData {
    pub namespace: String,
    pub cursor: Cursor,
    pub health: SnapshotHealth,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub default_profile: Option<String>,
    /// What the daemon resolved as the default, published whether or not the
    /// snapshot carries it. `default_profile` says whether that name resolves;
    /// this says which name, so a client that has to refuse can name the
    /// profile the user has to create.
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub resolved_default_profile: Option<String>,
    pub profiles: Vec<ProfileRead>,
    pub sessions: Vec<SessionRead>,
    pub global_projects: Vec<ProjectRead>,
    pub status_freshness: StatusFreshness,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Cursor {
    pub epoch: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Owner {
    pub kind: OwnerKind,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub uid: Option<u32>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OwnerKind {
    LocalOwner,
    Remote,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum AggregateHealth {
    Healthy,
    Degraded { code: HealthCode },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotHealth {
    pub global_enumeration: ComponentHealth,
    pub global_metadata: ComponentHealth,
    #[serde(deserialize_with = "deserialize_profile_health_map")]
    pub profiles: BTreeMap<String, ProfileHealth>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ComponentHealth {
    Healthy,
    Degraded { code: HealthCode },
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HealthCode {
    Enumeration,
    ProfileEnumeration,
    Metadata,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileHealth {
    pub profile_enumeration: ComponentHealth,
    pub metadata: ComponentHealth,
    pub profile_data: ComponentHealth,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum StatusFreshness {
    Unobserved {
        revision: u64,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        observed_at: Option<String>,
    },
    Observed {
        revision: u64,
        observed_at: String,
    },
    Unavailable {
        #[serde(deserialize_with = "deserialize_required_nullable")]
        revision: Option<u64>,
        #[serde(deserialize_with = "deserialize_required_nullable")]
        observed_at: Option<String>,
    },
}

/// What the Hello says about one profile: its name, and whether the daemon
/// could read it at all. The inventory is not repeated, because the Snapshot
/// that follows carries every group and project of every profile and each of
/// them is validated there.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileHello {
    pub name: String,
    pub listed: bool,
    pub aliases: Vec<String>,
    pub health: ProfileHealth,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProfileRead {
    pub name: String,
    pub listed: bool,
    pub aliases: Vec<String>,
    pub groups: Vec<GroupRead>,
    pub projects: Vec<ProjectRead>,
    pub health: ProfileHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GroupRead {
    pub name: String,
    pub path: String,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectRead {
    pub name: String,
    pub path: String,
    /// Required daemon-computed equality key, never rendered or resolved on the client.
    pub merge_key: String,
    pub scope: ProjectScope,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub default_base_branch: Option<String>,
    /// Registry membership is required; synthesized rows need a scoped session reference.
    pub registered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ProjectScope {
    Global,
    Profile,
}

/// CLI projection shared by producer and consumer. Commands and alias-only
/// profiles use the daemon-wide credential gate, not a per-profile ACL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SessionRead {
    pub id: String,
    pub title: String,
    pub project_path: String,
    pub group_path: String,
    pub tool: String,
    pub command: String,
    pub profile: String,
    pub status: WireStatus,
    pub state: WireState,
    pub created_at: String,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub last_accessed_at: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub idle_entered_at: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub last_error: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub archived_at: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub trashed_at: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub active_snoozed_until: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub pinned_at: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub agent_session_id: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub parent_session_id: Option<String>,
    pub has_worktree_info: bool,
    pub has_managed_worktree: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub worktree: Option<WorktreeRead>,
    pub workspace_repos: Vec<WorkspaceRepo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) enum WireStatus {
    Running,
    Waiting,
    Idle,
    Unknown,
    Stopped,
    Error,
    Starting,
    Deleting,
    Creating,
}

impl From<crate::session::Status> for WireStatus {
    fn from(status: crate::session::Status) -> Self {
        use crate::session::Status;
        match status {
            Status::Running => Self::Running,
            Status::Waiting => Self::Waiting,
            Status::Idle => Self::Idle,
            Status::Unknown => Self::Unknown,
            Status::Stopped => Self::Stopped,
            Status::Error => Self::Error,
            Status::Starting => Self::Starting,
            Status::Deleting => Self::Deleting,
            Status::Creating => Self::Creating,
        }
    }
}

impl WireStatus {
    pub(crate) fn as_str(self) -> &'static str {
        self.session_status().wire_str()
    }
    /// CLI JSON status values are lowercase; human status and wire values are PascalCase.
    pub(crate) fn json_str(self) -> &'static str {
        self.session_status().as_str()
    }

    /// Domain status used by the shared CLI predicates and tallies.
    pub(crate) fn session_status(self) -> crate::session::Status {
        use crate::session::Status;
        match self {
            Self::Running => Status::Running,
            Self::Waiting => Status::Waiting,
            Self::Idle => Status::Idle,
            Self::Unknown => Status::Unknown,
            Self::Stopped => Status::Stopped,
            Self::Error => Status::Error,
            Self::Starting => Status::Starting,
            Self::Deleting => Status::Deleting,
            Self::Creating => Status::Creating,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WireState {
    Live,
    Archived,
    Trashed,
}

impl From<crate::session::SessionBucket> for WireState {
    fn from(bucket: crate::session::SessionBucket) -> Self {
        use crate::session::SessionBucket;
        match bucket {
            SessionBucket::Active => Self::Live,
            SessionBucket::Archived => Self::Archived,
            SessionBucket::Trashed => Self::Trashed,
        }
    }
}

impl WireState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Archived => "archived",
            Self::Trashed => "trashed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorktreeRead {
    pub branch: String,
    pub main_repo_path: String,
    pub managed_by_aoe: bool,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub base_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceRepo {
    pub name: String,
    pub source_path: String,
    pub branch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum ApplicationFrame {
    Hello(HelloData),
    Snapshot(SnapshotData),
}

pub(crate) fn parse_hello(bytes: &[u8]) -> Result<HelloData, HelloParseError> {
    if let Ok(probe) = serde_json::from_slice::<HelloVersionProbe>(bytes) {
        if probe.protocol_version != PROTOCOL_VERSION {
            return Err(HelloParseError::ProtocolVersion);
        }
    }
    match serde_json::from_slice::<ApplicationFrame>(bytes) {
        Ok(ApplicationFrame::Hello(data)) => Ok(data),
        Ok(ApplicationFrame::Snapshot(_)) => Err(HelloParseError::Schema),
        Err(_) => Err(HelloParseError::Schema),
    }
}

pub(crate) fn parse_snapshot(bytes: &[u8]) -> Result<SnapshotData, ()> {
    match serde_json::from_slice::<ApplicationFrame>(bytes) {
        Ok(ApplicationFrame::Snapshot(data)) => Ok(data),
        _ => Err(()),
    }
}

#[derive(Debug)]
pub(crate) enum HelloParseError {
    ProtocolVersion,
    Schema,
}

struct HelloVersionProbe {
    protocol_version: u16,
}

impl<'de> Deserialize<'de> for HelloVersionProbe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProbeVisitor;

        impl<'de> Visitor<'de> for ProbeVisitor {
            type Value = HelloVersionProbe;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a hello envelope")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut kind: Option<String> = None;
                let mut data: Option<serde_json::Value> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "kind" if kind.replace(map.next_value()?).is_none() => {}
                        "data" if data.replace(map.next_value()?).is_none() => {}
                        _ => return Err(de::Error::duplicate_field("field")),
                    }
                }
                if kind.as_deref() != Some("hello") {
                    return Err(de::Error::custom("not a hello frame"));
                }
                let data = data.ok_or_else(|| de::Error::missing_field("data"))?;
                let mut object = match data {
                    serde_json::Value::Object(object) => object,
                    _ => return Err(de::Error::custom("hello data is not an object")),
                };
                let version = object
                    .remove("protocol_version")
                    .ok_or_else(|| de::Error::missing_field("protocol_version"))?;
                Ok(HelloVersionProbe {
                    protocol_version: serde_json::from_value(version)
                        .map_err(|_| de::Error::custom("invalid protocol version"))?,
                })
            }
        }

        deserializer.deserialize_map(ProbeVisitor)
    }
}

fn deserialize_required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn deserialize_profile_health_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, ProfileHealth>, D::Error>
where
    D: Deserializer<'de>,
{
    struct MapVisitor;

    impl<'de> Visitor<'de> for MapVisitor {
        type Value = BTreeMap<String, ProfileHealth>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a map of profile health values")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut output = BTreeMap::new();
            while let Some(key) = map.next_key::<String>()? {
                let value = map.next_value()?;
                if output.insert(key.clone(), value).is_some() {
                    return Err(de::Error::custom(format!(
                        "duplicate profile health key {key:?}"
                    )));
                }
            }
            Ok(output)
        }
    }

    deserializer.deserialize_map(MapVisitor)
}

pub(crate) fn valid_namespace(value: &str) -> bool {
    let Some((kind, suffix)) = value.split_once(':') else {
        return false;
    };
    matches!(kind, "debug" | "release")
        && (1..=200).contains(&suffix.len())
        && suffix.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
}

pub(crate) fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
        })
}

pub(crate) fn validate_hello(hello: &HelloData) -> Result<(), &'static str> {
    if hello.protocol_version != PROTOCOL_VERSION {
        return Err("protocol_mismatch");
    }
    if !valid_namespace(&hello.namespace)
        || !valid_uuid(&hello.runtime_epoch)
        || !valid_uuid(&hello.prebind_instance_id)
        || !valid_uuid(&hello.runtime_instance_id)
    {
        return Err("schema_invalid");
    }
    match (hello.local_owner, hello.owner.kind, hello.owner.uid) {
        (true, OwnerKind::LocalOwner, Some(_)) => {}
        (false, OwnerKind::Remote, None) => {}
        _ => return Err("schema_invalid"),
    }
    validate_profile_hellos(&hello.profiles)?;
    validate_freshness(&hello.status_freshness)?;
    if let AggregateHealth::Degraded { code } = hello.health {
        validate_health_code(code, &hello_health_codes())?;
    }
    Ok(())
}

/// Bound ancestry walks on session rows.
const MAX_SESSION_DEPTH: usize = 64;
pub(crate) fn validate_snapshot(snapshot: &SnapshotData) -> Result<(), &'static str> {
    if !valid_namespace(&snapshot.namespace)
        || !valid_uuid(&snapshot.cursor.epoch)
        || snapshot.cursor.revision == 0
    {
        return Err("schema_invalid");
    }
    validate_global_health(snapshot.health.global_enumeration)?;
    validate_global_health(snapshot.health.global_metadata)?;
    validate_profiles(&snapshot.profiles)?;
    if snapshot.health.profiles.len() != snapshot.profiles.len() {
        return Err("schema_invalid");
    }
    for profile in &snapshot.profiles {
        let health = snapshot
            .health
            .profiles
            .get(&profile.name)
            .ok_or("schema_invalid")?;
        // The map and profile row must describe the same load.
        if *health != profile.health {
            return Err("schema_invalid");
        }
        validate_profile_health(&profile.health)?;
    }
    // `default_profile` names a row this snapshot carries or names nothing.
    // The resolved name beside it may name no row, and that is not a schema
    // fault: it is the state a client reports by name.
    if let Some(default) = &snapshot.default_profile {
        if !snapshot
            .profiles
            .iter()
            .any(|profile| &profile.name == default || profile.aliases.contains(default))
        {
            return Err("schema_invalid");
        }
    }
    if let Some(resolved) = &snapshot.resolved_default_profile {
        if resolved.is_empty() {
            return Err("schema_invalid");
        }
    }

    validate_freshness(&snapshot.status_freshness)?;
    validate_projects(&snapshot.global_projects, ProjectScope::Global)?;
    let mut profile_groups: HashMap<&str, HashSet<&str>> = HashMap::new();
    let mut profile_projects: HashMap<&str, HashSet<&str>> = HashMap::new();
    // Every project path a session of that profile uses, so an unregistered
    // project row can be checked against the row that justifies it.
    let mut profile_session_paths: HashMap<&str, HashSet<&str>> = HashMap::new();
    for session in &snapshot.sessions {
        profile_session_paths
            .entry(session.profile.as_str())
            .or_default()
            .insert(session.project_path.as_str());
    }
    for profile in &snapshot.profiles {
        profile_groups.insert(
            profile.name.as_str(),
            profile
                .groups
                .iter()
                .map(|group| group.path.as_str())
                .collect(),
        );
        profile_projects.insert(
            profile.name.as_str(),
            profile
                .projects
                .iter()
                .map(|project| project.path.as_str())
                .collect(),
        );
        // An unregistered row exists only to make a session's project path
        // resolve, so a session row of this profile must name it. Without
        // that, a server could withhold a registry project by relabelling it.
        let used = profile_session_paths
            .get(profile.name.as_str())
            .cloned()
            .unwrap_or_default();
        if profile
            .projects
            .iter()
            .any(|project| !project.registered && !used.contains(project.path.as_str()))
        {
            return Err("schema_invalid");
        }
    }
    let global_projects: HashSet<&str> = snapshot
        .global_projects
        .iter()
        .map(|project| project.path.as_str())
        .collect();
    if !invalid_session_profiles(&snapshot.sessions).is_empty() {
        return Err("schema_invalid");
    }
    for session in &snapshot.sessions {
        let groups = profile_groups
            .get(session.profile.as_str())
            .ok_or("schema_invalid")?;
        let projects = profile_projects
            .get(session.profile.as_str())
            .ok_or("schema_invalid")?;
        if (!session.group_path.is_empty() && !groups.contains(session.group_path.as_str()))
            || (!projects.contains(session.project_path.as_str())
                && !global_projects.contains(session.project_path.as_str()))
        {
            return Err("schema_invalid");
        }
    }
    Ok(())
}

/// Shared by producer admission and validation of arbitrary peers.
pub(crate) fn invalid_session_profiles(sessions: &[SessionRead]) -> HashSet<&str> {
    let mut rejected = HashSet::new();
    let mut parents: HashMap<(&str, &str), Option<&str>> = HashMap::with_capacity(sessions.len());
    for session in sessions {
        if parents
            .insert(
                (session.profile.as_str(), session.id.as_str()),
                session.parent_session_id.as_deref(),
            )
            .is_some()
            || validate_session(session).is_err()
        {
            rejected.insert(session.profile.as_str());
        }
    }
    let mut seen = HashSet::new();
    for ((profile, id), parent) in &parents {
        if parent.is_none() || rejected.contains(profile) {
            continue;
        }
        seen.clear();
        let mut cursor = Some(*id);
        let mut depth = 0;
        while let Some(node) = cursor {
            if !seen.insert(node) || depth >= MAX_SESSION_DEPTH {
                rejected.insert(*profile);
                break;
            }
            depth += 1;
            cursor = parents.get(&(*profile, node)).copied().flatten();
        }
    }
    rejected
}

pub(crate) fn validate_cross_message(
    hello: &HelloData,
    snapshot: &SnapshotData,
    local_owner: Option<u32>,
) -> Result<(), &'static str> {
    if hello.runtime_epoch != snapshot.cursor.epoch {
        return Err("schema_invalid");
    }
    if hello.profiles.len() != snapshot.profiles.len()
        || hello
            .profiles
            .iter()
            .zip(&snapshot.profiles)
            .any(|(hello, snapshot)| {
                hello.name != snapshot.name
                    || hello.listed != snapshot.listed
                    || hello.aliases != snapshot.aliases
                    || hello.health != snapshot.health
            })
    {
        return Err("schema_invalid");
    }
    match local_owner {
        Some(uid) => {
            if hello.namespace != snapshot.namespace
                || !hello.local_owner
                || hello.owner.kind != OwnerKind::LocalOwner
                || hello.owner.uid != Some(uid)
            {
                return Err("peer_identity");
            }
        }
        None => {
            if hello.namespace != snapshot.namespace
                || hello.local_owner
                || hello.owner.kind != OwnerKind::Remote
                || hello.owner.uid.is_some()
            {
                return Err("schema_invalid");
            }
        }
    }
    Ok(())
}

/// Profiles in the producer's own order, which is the order a local
/// `aoe profile` prints them in. The wire therefore carries presentation, and
/// the identity rule is uniqueness rather than a canonical sort.
fn validate_profiles(profiles: &[ProfileRead]) -> Result<(), &'static str> {
    let mut names = HashSet::new();
    let mut selectable: HashSet<&str> = profiles
        .iter()
        .map(|profile| profile.name.as_str())
        .collect();
    for profile in profiles {
        if !names.insert(profile.name.as_str()) {
            return Err("schema_invalid");
        }
        if profile.name.is_empty()
            || (!profile.listed && !crate::session::valid_profile_name(&profile.name))
        {
            return Err("schema_invalid");
        }
        validate_aliases(&profile.aliases, &mut selectable)?;
        validate_profile_health(&profile.health)?;
        validate_groups(&profile.groups)?;
        validate_projects(&profile.projects, ProjectScope::Profile)?;
    }
    Ok(())
}

/// Hello names and aliases are correlated with the complete snapshot.
fn validate_profile_hellos(profiles: &[ProfileHello]) -> Result<(), &'static str> {
    let mut names = HashSet::new();
    let mut selectable: HashSet<&str> = profiles
        .iter()
        .map(|profile| profile.name.as_str())
        .collect();
    for profile in profiles {
        if !names.insert(profile.name.as_str()) {
            return Err("schema_invalid");
        }
        if profile.name.is_empty()
            || (!profile.listed && !crate::session::valid_profile_name(&profile.name))
        {
            return Err("schema_invalid");
        }
        validate_aliases(&profile.aliases, &mut selectable)?;
        validate_profile_health(&profile.health)?;
    }
    Ok(())
}

fn validate_aliases<'a>(
    aliases: &'a [String],
    names: &mut HashSet<&'a str>,
) -> Result<(), &'static str> {
    for alias in aliases {
        if !crate::session::valid_profile_name(alias) || !names.insert(alias.as_str()) {
            return Err("schema_invalid");
        }
    }
    Ok(())
}

fn validate_profile_health(health: &ProfileHealth) -> Result<(), &'static str> {
    validate_profile_component(health.profile_enumeration)?;
    validate_profile_component(health.metadata)?;
    validate_profile_component(health.profile_data)
}

/// One health code against the components that may actually carry it. The
/// client's vocabulary is the producer's, and each component is degraded by
/// its own failure, so a code outside the component's own set is a schema
/// violation rather than a fact about the daemon.
fn validate_health_code(code: HealthCode, allowed: &[HealthCode]) -> Result<(), &'static str> {
    if allowed.contains(&code) {
        Ok(())
    } else {
        Err("schema_invalid")
    }
}

/// The codes a global component is degraded by.
const GLOBAL_HEALTH_CODES: &[HealthCode] = &[HealthCode::Enumeration, HealthCode::Metadata];

/// The codes a profile-scoped component is degraded by.
const PROFILE_COMPONENT_CODES: &[HealthCode] =
    &[HealthCode::ProfileEnumeration, HealthCode::Metadata];

/// The aggregate a Hello may carry: exactly the union of the codes the
/// components folded into it can each carry, so the roll-up cannot name a code
/// the components it rolls up are incapable of. Derived rather than written
/// out, because a hand-written subset of the union is a way to refuse a Hello
/// the producer is entitled to send.
fn hello_health_codes() -> Vec<HealthCode> {
    let mut codes: Vec<HealthCode> = Vec::new();
    for component in [GLOBAL_HEALTH_CODES, PROFILE_COMPONENT_CODES] {
        for code in component {
            if !codes.contains(code) {
                codes.push(*code);
            }
        }
    }
    codes
}

fn validate_global_health(health: ComponentHealth) -> Result<(), &'static str> {
    if let ComponentHealth::Degraded { code } = health {
        validate_health_code(code, GLOBAL_HEALTH_CODES)?;
    }
    Ok(())
}

fn validate_profile_component(health: ComponentHealth) -> Result<(), &'static str> {
    if let ComponentHealth::Degraded { code } = health {
        validate_health_code(code, PROFILE_COMPONENT_CODES)?;
    }
    Ok(())
}

/// Preserve registry order and validate group-tree references, not filesystem syntax.
pub(crate) fn validate_groups(groups: &[GroupRead]) -> Result<(), &'static str> {
    let mut unique: HashSet<&str> = HashSet::new();
    for group in groups {
        if !unique.insert(group.path.as_str()) {
            return Err("schema_invalid");
        }
    }
    let paths = unique;
    for group in groups {
        let expected_name = group.path.rsplit('/').next().unwrap_or_default();
        if group.name != expected_name {
            return Err("schema_invalid");
        }
        // Child lists may be incomplete, but every named child must exist.
        let mut child_names: HashSet<&str> = HashSet::new();
        for child in &group.children {
            let child_path = format!("{}/{}", group.path, child);
            if !child_names.insert(child.as_str()) || !paths.contains(child_path.as_str()) {
                return Err("schema_invalid");
            }
        }
    }
    Ok(())
}

/// Validate project scope, paths and exact registry identities.
pub(crate) fn validate_projects(
    projects: &[ProjectRead],
    scope: ProjectScope,
) -> Result<(), &'static str> {
    let mut identities = HashSet::new();
    for project in projects {
        if project.scope as u8 != scope as u8
            // The global list is the global registry, so every row in it is
            // registered by definition; only a profile list may hold a
            // synthesized row.
            || (matches!(scope, ProjectScope::Global) && !project.registered)
            || !(if project.registered {
                valid_stored_project_path(&project.path)
            } else {
                valid_session_project_path(&project.path)
            })
            || (project.merge_key.is_empty()
                && (project.registered || !project.path.is_empty()))
            || !identities.insert((
                project.name.as_str(),
                project.path.as_str(),
                project.registered,
            ))
        {
            return Err("schema_invalid");
        }
    }
    Ok(())
}

fn validate_session(session: &SessionRead) -> Result<(), &'static str> {
    // Stored display text is preserved; identifiers retain their own checks.
    validate_safe_text(&session.id)?;
    if session.profile.is_empty() {
        return Err("schema_invalid");
    }
    if session
        .agent_session_id
        .as_deref()
        .is_some_and(|value| !valid_text(value))
        || session
            .parent_session_id
            .as_deref()
            .is_some_and(|value| !valid_text(value))
    {
        return Err("schema_invalid");
    }
    if !valid_session_project_path(&session.project_path) {
        return Err("schema_invalid");
    }
    for value in [
        session.last_accessed_at.as_deref(),
        session.idle_entered_at.as_deref(),
        session.archived_at.as_deref(),
        session.trashed_at.as_deref(),
        session.active_snoozed_until.as_deref(),
        session.pinned_at.as_deref(),
    ] {
        if value.is_some_and(|value| !valid_timestamp(value)) {
            return Err("schema_invalid");
        }
    }
    if !valid_timestamp(&session.created_at) {
        return Err("schema_invalid");
    }
    match session.state {
        WireState::Live if session.archived_at.is_some() || session.trashed_at.is_some() => {
            return Err("schema_invalid")
        }
        WireState::Archived if session.archived_at.is_none() || session.trashed_at.is_some() => {
            return Err("schema_invalid")
        }
        WireState::Trashed if session.trashed_at.is_none() => return Err("schema_invalid"),
        _ => {}
    }
    if session.has_worktree_info != session.worktree.is_some() {
        return Err("schema_invalid");
    }
    if session.has_managed_worktree && session.worktree.is_none() {
        return Err("schema_invalid");
    }
    if let Some(worktree) = &session.worktree {
        if session.has_managed_worktree != worktree.managed_by_aoe
            || !valid_stored_project_path(&worktree.main_repo_path)
        {
            return Err("schema_invalid");
        }
    }
    // Preserve workspace order and native basenames; identity is (name, source_path).
    let mut repo_identities: HashSet<(&str, &str)> = HashSet::new();
    for repository in &session.workspace_repos {
        if !valid_absolute_path(&repository.source_path)
            || !repo_identities.insert((&repository.name, &repository.source_path))
        {
            return Err("schema_invalid");
        }
    }
    Ok(())
}

fn validate_freshness(freshness: &StatusFreshness) -> Result<(), &'static str> {
    match freshness {
        StatusFreshness::Unobserved {
            revision,
            observed_at,
        } => (*revision == 0 && observed_at.is_none())
            .then_some(())
            .ok_or("schema_invalid"),
        StatusFreshness::Observed {
            revision,
            observed_at,
        } => (*revision != 0 && valid_timestamp(observed_at))
            .then_some(())
            .ok_or("schema_invalid"),
        StatusFreshness::Unavailable {
            revision,
            observed_at,
        } => (revision.is_none() && observed_at.is_none())
            .then_some(())
            .ok_or("schema_invalid"),
    }
}

pub(crate) fn freshness_observed(freshness: &StatusFreshness) -> bool {
    matches!(freshness, StatusFreshness::Observed { .. })
}

pub(crate) fn component_healthy(health: &ComponentHealth) -> bool {
    matches!(health, ComponentHealth::Healthy)
}

pub(crate) fn profile_component_healthy(health: &ComponentHealth) -> bool {
    component_healthy(health)
}

fn valid_text(value: &str) -> bool {
    !value.chars().any(is_c1_c0)
}

fn validate_safe_text(value: &str) -> Result<(), &'static str> {
    (!value.is_empty() && valid_text(value))
        .then_some(())
        .ok_or("schema_invalid")
}

fn is_c1_c0(value: char) -> bool {
    matches!(
        value as u32,
        0x00..=0x1f | 0x7f..=0x9f | 0x2028..=0x2029 | 0x202a..=0x202e | 0x2066..=0x2069
    )
}

/// `YYYY-MM-DDTHH:MM:SS[.f{3,6,9}]Z`: UTC, `Z` zoned, and either a whole
/// second or the three fractional widths chrono's `AutoSi` writes. The fixed
/// separators are checked by position and the value must parse, so a
/// permissive length is the only thing that is relaxed: a control character, a
/// numeric offset or a lowercase `t`/`z` is still refused.
///
/// Those widths are the producer's, not RFC 3339's: every instant on this
/// wire is spelled with `SecondsFormat::AutoSi`, so any other count is a frame
/// this half would echo as a spelling the local command never prints.
fn valid_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    // The head is 19 bytes: a whole second puts its `Z` at index 19, and a
    // fractional part puts a `.` there before the final `Z`.
    let fraction: &[u8] = match bytes.len() {
        20 => &[],
        length if length > 21 => match bytes[19] {
            b'.' => &bytes[20..length - 1],
            _ => return false,
        },
        _ => return false,
    };
    if !matches!(fraction.len(), 0 | 3 | 6 | 9) || !fraction.iter().all(u8::is_ascii_digit) {
        return false;
    }
    value.ends_with('Z') && chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

/// Canonical absolute POSIX spelling; native components may contain controls, but not NUL.
pub(crate) fn valid_absolute_path(value: &str) -> bool {
    value == "/"
        || (value.starts_with('/')
            && value.split('/').skip(1).all(|component| {
                !component.is_empty()
                    && component != "."
                    && component != ".."
                    && !component.contains('\0')
            }))
}

/// Stored absolute spellings are opaque and need not be lexically normalized.
pub(crate) fn valid_stored_project_path(value: &str) -> bool {
    value.starts_with('/') && !value.contains('\0')
}

/// The existing session writer uses an empty string for a non-UTF-8 native path.
pub(crate) fn valid_session_project_path(value: &str) -> bool {
    value.is_empty() || valid_stored_project_path(value)
}

