use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::detect::Agent;

const MAX_SIDEBAR_ROWS: usize = 16;
const MAX_SIDEBAR_TOKENS_PER_ROW: usize = 16;
const DEFAULT_SIDEBAR_ROW_GAP: u16 = 0;
const DEFAULT_RIGHT_PANEL_WIDTH: u16 = 24;

fn deserialize_sidebar_rows<'de, D, T>(deserializer: D) -> Result<Vec<Vec<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let rows = Vec::<Vec<T>>::deserialize(deserializer)?;
    validate_sidebar_rows(&rows).map_err(serde::de::Error::custom)?;
    Ok(rows)
}

pub(crate) fn validate_sidebar_rows<T>(rows: &[Vec<T>]) -> Result<(), String> {
    if rows.len() > MAX_SIDEBAR_ROWS {
        return Err(format!(
            "sidebar layouts may contain at most {MAX_SIDEBAR_ROWS} rows"
        ));
    }
    if rows
        .iter()
        .any(|row| row.len() > MAX_SIDEBAR_TOKENS_PER_ROW)
    {
        return Err(format!(
            "sidebar rows may contain at most {MAX_SIDEBAR_TOKENS_PER_ROW} tokens"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidebarTokenColor {
    r: u8,
    g: u8,
    b: u8,
}

impl SidebarTokenColor {
    pub(crate) fn ratatui(self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.r, self.g, self.b)
    }
}

impl Serialize for SidebarTokenColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b))
    }
}

impl<'de> Deserialize<'de> for SidebarTokenColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let hex = value.strip_prefix('#').filter(|hex| {
            hex.is_ascii()
                && matches!(hex.len(), 3 | 6)
                && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
        let Some(hex) = hex else {
            return Err(serde::de::Error::custom(
                "sidebar token fg must be #RGB or #RRGGBB",
            ));
        };
        let (r, g, b) = if hex.len() == 3 {
            let mut digits = hex
                .bytes()
                .map(|byte| char::from(byte).to_digit(16).expect("validated hex digit") as u8 * 17);
            (
                digits.next().expect("three hex digits"),
                digits.next().expect("three hex digits"),
                digits.next().expect("three hex digits"),
            )
        } else {
            (
                u8::from_str_radix(&hex[0..2], 16).expect("validated hex digits"),
                u8::from_str_radix(&hex[2..4], 16).expect("validated hex digits"),
                u8::from_str_radix(&hex[4..6], 16).expect("validated hex digits"),
            )
        };
        Ok(Self { r, g, b })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SidebarTokenStyle {
    pub fg: Option<SidebarTokenColor>,
    pub bold: Option<bool>,
    pub dim: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSidebarToken {
    StateIcon,
    StateText,
    Workspace,
    Tab,
    Pane,
    Agent,
    TerminalTitle,
    TerminalTitleStripped,
    /// Cumulative spend, resolved by Shepherd's own session-provider refresh.
    Spend,
    /// Cumulative savings from prompt caching. `llmtrim` source.
    Savings,
    /// Cumulative cache-read tokens. `llmtrim` source.
    CacheReadTokens,
    /// Instruction-budget total bytes. `context-floor` source.
    InstructionTotalBytes,
    /// Instruction-budget floor warning — renders only when the warning is active.
    /// `context-floor` source.
    InstructionFloorWarn,
    /// Instruction-budget growth percentage since the prior observation. `context-floor` source.
    InstructionGrowthPct,
    /// Instruction-budget dollars per 1,000 turns. `context-floor` source.
    InstructionDollarsPer1kTurns,
    /// The session's model name. `sessions` source.
    Model,
    /// The session's current context-window token count. `sessions` source.
    ContextTokens,
    /// The session's context-window size. `sessions` source.
    ContextWindowTokens,
    /// How many sessions are in the persisted session store. `sessions` source.
    SessionCount,
    /// Remaining quota across every account with fresh, complete usage data.
    /// `local-account-signal` source.
    QuotaRemaining,
    /// The earliest cooldown reset time among accounts on cooldown. `local-account-signal`
    /// source.
    ResetAt,
    Custom(String),
    Styled {
        token: Box<AgentSidebarToken>,
        style: SidebarTokenStyle,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceSidebarToken {
    StateIcon,
    StateText,
    Workspace,
    Branch,
    GitStatus,
    Proposals,
    Beads,
    Custom(String),
    Styled {
        token: Box<SpaceSidebarToken>,
        style: SidebarTokenStyle,
    },
}

impl AgentSidebarToken {
    pub(crate) fn parts(&self) -> (&Self, SidebarTokenStyle) {
        match self {
            Self::Styled { token, style } => (token, *style),
            token => (token, SidebarTokenStyle::default()),
        }
    }
}

impl SpaceSidebarToken {
    pub(crate) fn parts(&self) -> (&Self, SidebarTokenStyle) {
        match self {
            Self::Styled { token, style } => (token, *style),
            token => (token, SidebarTokenStyle::default()),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStyledSidebarToken {
    token: String,
    #[serde(default)]
    fg: Option<SidebarTokenColor>,
    #[serde(default)]
    bold: Option<bool>,
    #[serde(default)]
    dim: Option<bool>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawSidebarToken {
    Plain(String),
    Styled(RawStyledSidebarToken),
}

impl RawSidebarToken {
    fn parts(self) -> (String, Option<SidebarTokenStyle>) {
        match self {
            Self::Plain(token) => (token, None),
            Self::Styled(token) => (
                token.token,
                Some(SidebarTokenStyle {
                    fg: token.fg,
                    bold: token.bold,
                    dim: token.dim,
                }),
            ),
        }
    }
}

fn parse_sidebar_token<T>(value: String, builtins: &[(&str, T)]) -> Result<T, String>
where
    T: Clone + From<String>,
{
    if let Some((_, token)) = builtins.iter().find(|(name, _)| *name == value) {
        return Ok(token.clone());
    }
    let Some(name) = value.strip_prefix('$') else {
        return Err(format!(
            "unknown sidebar token `{value}`; custom tokens must start with `$`"
        ));
    };
    if name.is_empty()
        || name.len() > 32
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        return Err(format!("invalid custom sidebar token `{value}`"));
    }
    Ok(T::from(name.to_string()))
}

fn serialize_styled_token<S>(
    name: String,
    style: SidebarTokenStyle,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(None)?;
    map.serialize_entry("token", &name)?;
    if let Some(fg) = style.fg {
        map.serialize_entry("fg", &fg)?;
    }
    if let Some(bold) = style.bold {
        map.serialize_entry("bold", &bold)?;
    }
    if let Some(dim) = style.dim {
        map.serialize_entry("dim", &dim)?;
    }
    map.end()
}

fn agent_token_name(token: &AgentSidebarToken) -> String {
    match token {
        AgentSidebarToken::StateIcon => "state_icon".into(),
        AgentSidebarToken::StateText => "state_text".into(),
        AgentSidebarToken::Workspace => "workspace".into(),
        AgentSidebarToken::Tab => "tab".into(),
        AgentSidebarToken::Pane => "pane".into(),
        AgentSidebarToken::Agent => "agent".into(),
        AgentSidebarToken::TerminalTitle => "terminal_title".into(),
        AgentSidebarToken::TerminalTitleStripped => "terminal_title_stripped".into(),
        AgentSidebarToken::Spend => "spend".into(),
        AgentSidebarToken::Savings => "savings".into(),
        AgentSidebarToken::CacheReadTokens => "cache_read_tokens".into(),
        AgentSidebarToken::InstructionTotalBytes => "instruction_total_bytes".into(),
        AgentSidebarToken::InstructionFloorWarn => "instruction_floor_warn".into(),
        AgentSidebarToken::InstructionGrowthPct => "instruction_growth_pct".into(),
        AgentSidebarToken::InstructionDollarsPer1kTurns => {
            "instruction_dollars_per_1k_turns".into()
        }
        AgentSidebarToken::Model => "model".into(),
        AgentSidebarToken::ContextTokens => "context_tokens".into(),
        AgentSidebarToken::ContextWindowTokens => "context_window_tokens".into(),
        AgentSidebarToken::SessionCount => "session_count".into(),
        AgentSidebarToken::QuotaRemaining => "quota_remaining".into(),
        AgentSidebarToken::ResetAt => "reset_at".into(),
        AgentSidebarToken::Custom(name) => format!("${name}"),
        AgentSidebarToken::Styled { token, .. } => agent_token_name(token),
    }
}

fn space_token_name(token: &SpaceSidebarToken) -> String {
    match token {
        SpaceSidebarToken::StateIcon => "state_icon".into(),
        SpaceSidebarToken::StateText => "state_text".into(),
        SpaceSidebarToken::Workspace => "workspace".into(),
        SpaceSidebarToken::Branch => "branch".into(),
        SpaceSidebarToken::GitStatus => "git_status".into(),
        SpaceSidebarToken::Proposals => "proposals".into(),
        SpaceSidebarToken::Beads => "beads".into(),
        SpaceSidebarToken::Custom(name) => format!("${name}"),
        SpaceSidebarToken::Styled { token, .. } => space_token_name(token),
    }
}

impl Serialize for AgentSidebarToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Styled { token, style } => {
                serialize_styled_token(agent_token_name(token), *style, serializer)
            }
            token => serializer.serialize_str(&agent_token_name(token)),
        }
    }
}

impl From<String> for AgentSidebarToken {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}

impl<'de> Deserialize<'de> for AgentSidebarToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (value, style) = RawSidebarToken::deserialize(deserializer)?.parts();
        let token = parse_sidebar_token(
            value,
            &[
                ("state_icon", Self::StateIcon),
                ("state_text", Self::StateText),
                ("workspace", Self::Workspace),
                ("tab", Self::Tab),
                ("pane", Self::Pane),
                ("agent", Self::Agent),
                ("terminal_title", Self::TerminalTitle),
                ("terminal_title_stripped", Self::TerminalTitleStripped),
                ("spend", Self::Spend),
                ("savings", Self::Savings),
                ("cache_read_tokens", Self::CacheReadTokens),
                ("instruction_total_bytes", Self::InstructionTotalBytes),
                ("instruction_floor_warn", Self::InstructionFloorWarn),
                ("instruction_growth_pct", Self::InstructionGrowthPct),
                (
                    "instruction_dollars_per_1k_turns",
                    Self::InstructionDollarsPer1kTurns,
                ),
                ("model", Self::Model),
                ("context_tokens", Self::ContextTokens),
                ("context_window_tokens", Self::ContextWindowTokens),
                ("session_count", Self::SessionCount),
                ("quota_remaining", Self::QuotaRemaining),
                ("reset_at", Self::ResetAt),
            ],
        )
        .map_err(serde::de::Error::custom)?;
        Ok(style.map_or(token.clone(), |style| Self::Styled {
            token: Box::new(token),
            style,
        }))
    }
}

impl Serialize for SpaceSidebarToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Styled { token, style } => {
                serialize_styled_token(space_token_name(token), *style, serializer)
            }
            token => serializer.serialize_str(&space_token_name(token)),
        }
    }
}

impl From<String> for SpaceSidebarToken {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}

impl<'de> Deserialize<'de> for SpaceSidebarToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (value, style) = RawSidebarToken::deserialize(deserializer)?.parts();
        let token = parse_sidebar_token(
            value,
            &[
                ("state_icon", Self::StateIcon),
                ("state_text", Self::StateText),
                ("workspace", Self::Workspace),
                ("branch", Self::Branch),
                ("git_status", Self::GitStatus),
                ("proposals", Self::Proposals),
                ("beads", Self::Beads),
            ],
        )
        .map_err(serde::de::Error::custom)?;
        Ok(style.map_or(token.clone(), |style| Self::Styled {
            token: Box::new(token),
            style,
        }))
    }
}

type AgentSidebarRows = Vec<Vec<AgentSidebarToken>>;
type SpaceSidebarRows = Vec<Vec<SpaceSidebarToken>>;

fn deserialize_rows_by_agent<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, AgentSidebarRows>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let rows_by_agent = BTreeMap::<String, AgentSidebarRows>::deserialize(deserializer)?;
    for (id, rows) in &rows_by_agent {
        if crate::detect::parse_canonical_agent_label(id).is_none() {
            return Err(serde::de::Error::custom(format!(
                "unknown canonical agent id `{id}` in sidebar rows_by_agent"
            )));
        }
        validate_sidebar_rows(rows).map_err(serde::de::Error::custom)?;
    }
    Ok(rows_by_agent)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct AgentsSidebarConfig {
    #[serde(deserialize_with = "deserialize_sidebar_rows")]
    pub rows: AgentSidebarRows,
    #[serde(default, deserialize_with = "deserialize_rows_by_agent")]
    pub rows_by_agent: BTreeMap<String, AgentSidebarRows>,
    pub row_gap: u16,
}

impl AgentsSidebarConfig {
    pub(crate) fn rows_for_agent(&self, agent: Option<Agent>) -> &AgentSidebarRows {
        agent
            .and_then(|agent| self.rows_by_agent.get(crate::detect::agent_label(agent)))
            .unwrap_or(&self.rows)
    }
}

impl Default for AgentsSidebarConfig {
    fn default() -> Self {
        Self {
            rows: vec![
                vec![
                    AgentSidebarToken::StateIcon,
                    AgentSidebarToken::Workspace,
                    AgentSidebarToken::Tab,
                ],
                vec![AgentSidebarToken::Agent],
            ],
            rows_by_agent: BTreeMap::new(),
            row_gap: DEFAULT_SIDEBAR_ROW_GAP,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct SpacesSidebarConfig {
    #[serde(deserialize_with = "deserialize_sidebar_rows")]
    pub rows: SpaceSidebarRows,
    pub row_gap: u16,
}

impl Default for SpacesSidebarConfig {
    fn default() -> Self {
        Self {
            rows: vec![
                vec![SpaceSidebarToken::StateIcon, SpaceSidebarToken::Workspace],
                vec![SpaceSidebarToken::Branch, SpaceSidebarToken::GitStatus],
            ],
            row_gap: DEFAULT_SIDEBAR_ROW_GAP,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct SidebarConfig {
    pub agents: AgentsSidebarConfig,
    pub spaces: SpacesSidebarConfig,
}

/// Right chrome panel. Modelled on `SpacesSidebarConfig` rather than `DockConfig`: it is a
/// token-driven region, not a terminal host, so it carries rows instead of a `side`/`size` pair.
/// Its rows share the topbar's `AgentSidebarToken` vocabulary — there is one vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RightPanelConfig {
    pub enabled: bool,
    pub width: u16,
    #[serde(deserialize_with = "deserialize_sidebar_rows")]
    pub rows: AgentSidebarRows,
}

impl Default for RightPanelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            width: DEFAULT_RIGHT_PANEL_WIDTH,
            rows: vec![vec![AgentSidebarToken::Workspace]],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_panel_defaults_to_disabled_and_reserves_no_width() {
        let config = crate::config::Config::default();
        assert!(
            !config.ui.right_panel.enabled,
            "an unset right panel must not render"
        );

        // A disabled panel reserves nothing regardless of its configured width, so the
        // default width is inert until `enabled` is set.
        let reserved = if config.ui.right_panel.enabled {
            config.ui.right_panel.width
        } else {
            0
        };
        assert_eq!(reserved, 0);
    }

    #[test]
    fn spend_token_parses_round_trips_and_is_rejected_when_misspelled() {
        let config: crate::config::Config = toml::from_str(
            r#"
[ui.topbar]
enabled = true
rows = [["workspace", { token = "spend", dim = true }]]
"#,
        )
        .expect("spend is a documented agent token");

        assert_eq!(
            config.ui.topbar.rows,
            vec![vec![
                AgentSidebarToken::Workspace,
                AgentSidebarToken::Styled {
                    token: Box::new(AgentSidebarToken::Spend),
                    style: SidebarTokenStyle {
                        dim: Some(true),
                        ..Default::default()
                    },
                },
            ]]
        );
        assert_eq!(agent_token_name(&AgentSidebarToken::Spend), "spend");

        // A bare unknown name must not silently degrade to an absent token at render time.
        assert!(
            toml::from_str::<crate::config::Config>(
                "[ui.topbar]\nenabled = true\nrows = [[\"spend_usd\"]]\n"
            )
            .is_err(),
            "an unknown bare token is a configuration error, not an elision"
        );
    }

    /// Every session-provider token added alongside `spend`: each name parses, round-trips
    /// through `agent_token_name`, and is distinct from every other token's name.
    #[test]
    fn session_provider_tokens_parse_and_round_trip() {
        let cases = [
            ("savings", AgentSidebarToken::Savings),
            ("cache_read_tokens", AgentSidebarToken::CacheReadTokens),
            (
                "instruction_total_bytes",
                AgentSidebarToken::InstructionTotalBytes,
            ),
            (
                "instruction_floor_warn",
                AgentSidebarToken::InstructionFloorWarn,
            ),
            (
                "instruction_growth_pct",
                AgentSidebarToken::InstructionGrowthPct,
            ),
            (
                "instruction_dollars_per_1k_turns",
                AgentSidebarToken::InstructionDollarsPer1kTurns,
            ),
            ("model", AgentSidebarToken::Model),
            ("context_tokens", AgentSidebarToken::ContextTokens),
            (
                "context_window_tokens",
                AgentSidebarToken::ContextWindowTokens,
            ),
            ("session_count", AgentSidebarToken::SessionCount),
            ("quota_remaining", AgentSidebarToken::QuotaRemaining),
            ("reset_at", AgentSidebarToken::ResetAt),
        ];

        let mut seen_names = std::collections::HashSet::new();
        for (name, token) in &cases {
            assert!(seen_names.insert(*name), "duplicate token name {name:?}");
            assert_eq!(agent_token_name(token), *name);

            let toml_body = format!("[ui.topbar]\nenabled = true\nrows = [[{name:?}]]\n");
            let config: crate::config::Config =
                toml::from_str(&toml_body).unwrap_or_else(|e| panic!("{name} must parse: {e}"));
            assert_eq!(
                config.ui.topbar.rows,
                vec![vec![token.clone()]],
                "{name} must round-trip to its own variant"
            );
        }
    }

    #[test]
    fn right_panel_parses_agent_tokens_and_rejects_unknown_fields() {
        let config: crate::config::Config = toml::from_str(
            r#"
[ui.right_panel]
enabled = true
width = 30
rows = [["state_icon", "workspace"], [{ token = "agent", bold = true }, "$model"]]
"#,
        )
        .expect("right panel config");

        assert!(config.ui.right_panel.enabled);
        assert_eq!(config.ui.right_panel.width, 30);
        assert_eq!(config.ui.right_panel.rows.len(), 2);
        assert_eq!(
            config.ui.right_panel.rows[0],
            vec![AgentSidebarToken::StateIcon, AgentSidebarToken::Workspace]
        );
        let (token, style) = config.ui.right_panel.rows[1][0].parts();
        assert_eq!(token, &AgentSidebarToken::Agent);
        assert_eq!(style.bold, Some(true));
        assert_eq!(
            config.ui.right_panel.rows[1][1],
            AgentSidebarToken::Custom("model".into())
        );

        let unknown = toml::from_str::<crate::config::Config>(
            "[ui.right_panel]\nenabled = true\nside = \"right\"\n",
        );
        assert!(
            unknown.is_err(),
            "a chrome panel takes no dock-style `side` field"
        );
    }

    #[test]
    fn defaults_match_the_compact_agent_and_existing_space_layouts() {
        let config = SidebarConfig::default();
        assert_eq!(
            config.agents.rows,
            vec![
                vec![
                    AgentSidebarToken::StateIcon,
                    AgentSidebarToken::Workspace,
                    AgentSidebarToken::Tab,
                ],
                vec![AgentSidebarToken::Agent],
            ]
        );
        assert!(config.agents.rows_by_agent.is_empty());
        assert_eq!(config.agents.row_gap, 0);
        assert_eq!(
            config.spaces.rows,
            vec![
                vec![SpaceSidebarToken::StateIcon, SpaceSidebarToken::Workspace],
                vec![SpaceSidebarToken::Branch, SpaceSidebarToken::GitStatus],
            ]
        );
        assert_eq!(config.spaces.row_gap, 0);
    }

    #[test]
    fn parses_builtin_and_arbitrary_custom_tokens() {
        let config: crate::config::Config = toml::from_str(
            r#"
[ui.sidebar.agents]
rows = [["state_icon", "workspace"], ["state_text", "agent", "$summary"], ["terminal_title", "terminal_title_stripped", "$terminal_title"]]
row_gap = 1

[ui.sidebar.agents.rows_by_agent]
claude = [["terminal_title_stripped"], ["agent", "$model"]]

[ui.sidebar.spaces]
rows = [["workspace"], ["$jj_status"]]
row_gap = 3
"#,
        )
        .expect("sidebar token config");

        assert_eq!(
            config.ui.sidebar.agents.rows[1],
            vec![
                AgentSidebarToken::StateText,
                AgentSidebarToken::Agent,
                AgentSidebarToken::Custom("summary".into()),
            ]
        );
        assert_eq!(
            config.ui.sidebar.agents.rows[2],
            vec![
                AgentSidebarToken::TerminalTitle,
                AgentSidebarToken::TerminalTitleStripped,
                AgentSidebarToken::Custom("terminal_title".into()),
            ]
        );
        assert_eq!(
            config.ui.sidebar.agents.rows_by_agent["claude"],
            vec![
                vec![AgentSidebarToken::TerminalTitleStripped],
                vec![
                    AgentSidebarToken::Agent,
                    AgentSidebarToken::Custom("model".into()),
                ],
            ]
        );
        assert_eq!(config.ui.sidebar.agents.row_gap, 1);
        assert_eq!(
            config.ui.sidebar.spaces.rows[1],
            vec![SpaceSidebarToken::Custom("jj_status".into())]
        );
        assert_eq!(config.ui.sidebar.spaces.row_gap, 3);
    }

    #[test]
    fn parses_occurrence_styles_without_changing_plain_tokens() {
        let config: crate::config::Config = toml::from_str(
            r##"
[ui.sidebar.agents]
rows = [[{ token = "workspace", fg = "#abc", bold = false }, "workspace"], [{ token = "$summary", dim = false }]]

[ui.sidebar.agents.rows_by_agent]
claude = [[{ token = "agent", fg = "#112233", bold = true, dim = false }]]

[ui.sidebar.spaces]
rows = [[{ token = "git_status", fg = "#ff00aa" }], [{ token = "$jj", bold = true }]]
"##,
        )
        .unwrap();

        let (token, style) = config.ui.sidebar.agents.rows[0][0].parts();
        assert_eq!(token, &AgentSidebarToken::Workspace);
        assert_eq!(style.bold, Some(false));
        assert_eq!(
            style.fg.unwrap().ratatui(),
            ratatui::style::Color::Rgb(0xaa, 0xbb, 0xcc)
        );
        assert_eq!(
            config.ui.sidebar.agents.rows[0][1],
            AgentSidebarToken::Workspace
        );

        let (token, style) = config.ui.sidebar.agents.rows_by_agent["claude"][0][0].parts();
        assert_eq!(token, &AgentSidebarToken::Agent);
        assert_eq!(style.bold, Some(true));
        assert_eq!(style.dim, Some(false));

        let (token, style) = config.ui.sidebar.spaces.rows[0][0].parts();
        assert_eq!(token, &SpaceSidebarToken::GitStatus);
        assert_eq!(
            style.fg.unwrap().ratatui(),
            ratatui::style::Color::Rgb(0xff, 0x00, 0xaa)
        );
        let (token, style) = config.ui.sidebar.spaces.rows[1][0].parts();
        assert_eq!(token, &SpaceSidebarToken::Custom("jj".into()));
        assert_eq!(style.bold, Some(true));
    }

    #[test]
    fn rejects_invalid_occurrence_styles() {
        for entry in [
            r##"{ token = "workspace", fg = "red" }"##,
            r##"{ token = "workspace", fg = "#abcd" }"##,
            r##"{ token = "workspace", underline = true }"##,
        ] {
            let input = format!("[ui.sidebar.agents]\nrows = [[{entry}]]\n");
            assert!(
                toml::from_str::<crate::config::Config>(&input).is_err(),
                "accepted {entry}"
            );
        }
    }

    #[test]
    fn rejects_unknown_bare_and_malformed_custom_tokens() {
        for token in ["summary", "$", "$bad.name"] {
            let input = format!("[ui.sidebar.agents]\\nrows = [[\"{token}\"]]\\n");
            assert!(toml::from_str::<crate::config::Config>(&input).is_err());
        }
    }

    #[test]
    fn rejects_oversized_sidebar_layouts() {
        let too_many_rows = std::iter::repeat_n("[\"agent\"]", MAX_SIDEBAR_ROWS + 1)
            .collect::<Vec<_>>()
            .join(",");
        let input = format!("[ui.sidebar.agents]\nrows = [{too_many_rows}]\n");
        assert!(toml::from_str::<crate::config::Config>(&input).is_err());

        let too_many_tokens = std::iter::repeat_n("\"workspace\"", MAX_SIDEBAR_TOKENS_PER_ROW + 1)
            .collect::<Vec<_>>()
            .join(",");
        let input = format!("[ui.sidebar.spaces]\nrows = [[{too_many_tokens}]]\n");
        assert!(toml::from_str::<crate::config::Config>(&input).is_err());

        let input = format!("[ui.sidebar.agents.rows_by_agent]\nclaude = [{too_many_rows}]\n");
        assert!(toml::from_str::<crate::config::Config>(&input).is_err());
    }

    #[test]
    fn accepts_every_canonical_agent_override_key() {
        let agents = [
            Agent::Pi,
            Agent::Claude,
            Agent::Codex,
            Agent::Gemini,
            Agent::Cursor,
            Agent::Devin,
            Agent::Antigravity,
            Agent::Cline,
            Agent::Omp,
            Agent::Mastracode,
            Agent::OpenCode,
            Agent::GithubCopilot,
            Agent::Kimi,
            Agent::Kiro,
            Agent::Droid,
            Agent::Amp,
            Agent::Grok,
            Agent::Hermes,
            Agent::Kilo,
            Agent::Qodercli,
            Agent::Maki,
        ];
        let entries = agents
            .iter()
            .map(|agent| format!("{} = [[\"agent\"]]", crate::detect::agent_label(*agent)))
            .collect::<Vec<_>>()
            .join("\n");
        let input = format!("[ui.sidebar.agents.rows_by_agent]\n{entries}\n");
        let config: crate::config::Config = toml::from_str(&input).expect("canonical keys");

        assert_eq!(config.ui.sidebar.agents.rows_by_agent.len(), agents.len());
    }

    #[test]
    fn rejects_alias_case_whitespace_and_unknown_override_keys() {
        for key in ["claude-code", "Claude", "' claude '", "unknown"] {
            let input = format!("[ui.sidebar.agents.rows_by_agent]\n{key} = [[\"agent\"]]\n");
            assert!(
                toml::from_str::<crate::config::Config>(&input).is_err(),
                "accepted key {key:?}"
            );
        }
    }
}
