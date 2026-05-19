// @generated — DO NOT EDIT MANUALLY.
// Regenerate with: bun run scripts/generate-schema-types.ts
// Source of truth: schema/v2.json

#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

#[doc = r" Error types."]
pub mod error {
    #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
#[doc = "Bun↔Node compatibility metadata of the host module — Phase 3+. Optional (rétro-compat). Le champ status pilote la sévérité dérivée."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Bun↔Node compatibility metadata of the host module — Phase 3+. Optional (rétro-compat). Le champ status pilote la sévérité dérivée.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"module\","]
#[doc = "    \"status\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"bunpp\": {"]
#[doc = "      \"description\": \"Polyfill @bun++/node-* recommandé quand status=missing. Optionnel.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"equivalent\": {"]
#[doc = "      \"description\": \"Équivalent Bun natif suggéré (ex: 'bun:sqlite', 'Bun.serve').\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"missing_apis\": {"]
#[doc = "      \"description\": \"Sous-APIs documentées comme non implémentées par Bun. Vide quand status=full.\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"module\": {"]
#[doc = "      \"description\": \"Nom du module Node concerné (sans le préfixe node:).\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"status\": {"]
#[doc = "      \"description\": \"Statut de couverture Bun du module Node host. full = tout passe, partial = bug-free sur les chemins courants, missing = pas d'équivalent natif.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"enum\": ["]
#[doc = "        \"full\","]
#[doc = "        \"partial\","]
#[doc = "        \"missing\""]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Compat {
    #[doc = "Polyfill @bun++/node-* recommandé quand status=missing. Optionnel."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub bunpp: ::std::option::Option<::std::string::String>,
    #[doc = "Équivalent Bun natif suggéré (ex: 'bun:sqlite', 'Bun.serve')."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub equivalent: ::std::option::Option<::std::string::String>,
    #[doc = "Sous-APIs documentées comme non implémentées par Bun. Vide quand status=full."]
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub missing_apis: ::std::vec::Vec<::std::string::String>,
    #[doc = "Nom du module Node concerné (sans le préfixe node:)."]
    pub module: ::std::string::String,
    #[doc = "Statut de couverture Bun du module Node host. full = tout passe, partial = bug-free sur les chemins courants, missing = pas d'équivalent natif."]
    pub status: CompatStatus,
}
impl Compat {
    pub fn builder() -> builder::Compat {
        Default::default()
    }
}
#[doc = "Statut de couverture Bun du module Node host. full = tout passe, partial = bug-free sur les chemins courants, missing = pas d'équivalent natif."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Statut de couverture Bun du module Node host. full = tout passe, partial = bug-free sur les chemins courants, missing = pas d'équivalent natif.\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"full\","]
#[doc = "    \"partial\","]
#[doc = "    \"missing\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum CompatStatus {
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "partial")]
    Partial,
    #[serde(rename = "missing")]
    Missing,
}
impl ::std::fmt::Display for CompatStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Full => f.write_str("full"),
            Self::Partial => f.write_str("partial"),
            Self::Missing => f.write_str("missing"),
        }
    }
}
impl ::std::str::FromStr for CompatStatus {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "full" => Ok(Self::Full),
            "partial" => Ok(Self::Partial),
            "missing" => Ok(Self::Missing),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for CompatStatus {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for CompatStatus {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for CompatStatus {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "Source context around a finding: up to 3 lines before, the finding's line, up to 3 lines after. Consumed by LLM/IDE integrations."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Source context around a finding: up to 3 lines before, the finding's line, up to 3 lines after. Consumed by LLM/IDE integrations.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"after\","]
#[doc = "    \"before\","]
#[doc = "    \"line\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"after\": {"]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"before\": {"]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"line\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub after: ::std::vec::Vec<::std::string::String>,
    pub before: ::std::vec::Vec<::std::string::String>,
    pub line: ::std::string::String,
}
impl Context {
    pub fn builder() -> builder::Context {
        Default::default()
    }
}
#[doc = "`FileFix`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"changed\","]
#[doc = "    \"findings\","]
#[doc = "    \"path\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"changed\": {"]
#[doc = "      \"description\": \"True if the file content differs from its pre-scan state (only in --fix / --aggressive / --migrate modes).\","]
#[doc = "      \"type\": \"boolean\""]
#[doc = "    },"]
#[doc = "    \"findings\": {"]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/Finding\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"path\": {"]
#[doc = "      \"description\": \"Relative path to the scanned root.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FileFix {
    #[doc = "True if the file content differs from its pre-scan state (only in --fix / --aggressive / --migrate modes)."]
    pub changed: bool,
    pub findings: ::std::vec::Vec<Finding>,
    #[doc = "Relative path to the scanned root."]
    pub path: ::std::string::String,
}
impl FileFix {
    pub fn builder() -> builder::FileFix {
        Default::default()
    }
}
#[doc = "`Finding`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"autofix\","]
#[doc = "    \"category\","]
#[doc = "    \"col\","]
#[doc = "    \"confidence\","]
#[doc = "    \"context\","]
#[doc = "    \"docs_url\","]
#[doc = "    \"end_byte\","]
#[doc = "    \"line\","]
#[doc = "    \"message\","]
#[doc = "    \"original\","]
#[doc = "    \"rule_id\","]
#[doc = "    \"severity\","]
#[doc = "    \"start_byte\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"aggressive\": {"]
#[doc = "      \"description\": \"True when the rule is only applied by --aggressive. Omitted when false/unset.\","]
#[doc = "      \"type\": \"boolean\""]
#[doc = "    },"]
#[doc = "    \"autofix\": {"]
#[doc = "      \"description\": \"True when the rule can be auto-applied by --fix.\","]
#[doc = "      \"type\": \"boolean\""]
#[doc = "    },"]
#[doc = "    \"category\": {"]
#[doc = "      \"description\": \"Top-level category derived from rule_id prefix.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"col\": {"]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"minimum\": 1.0"]
#[doc = "    },"]
#[doc = "    \"compat\": {"]
#[doc = "      \"description\": \"Phase 3+ : statut de compat Bun du module hôte. Présent uniquement sur les findings imports/node-* et api/node-*. Optionnel (rétro-compat schema_version=2).\","]
#[doc = "      \"$ref\": \"#/definitions/Compat\""]
#[doc = "    },"]
#[doc = "    \"confidence\": {"]
#[doc = "      \"description\": \"Heuristic confidence 0..1.\","]
#[doc = "      \"type\": \"number\","]
#[doc = "      \"maximum\": 1.0,"]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"context\": {"]
#[doc = "      \"$ref\": \"#/definitions/Context\""]
#[doc = "    },"]
#[doc = "    \"docs_url\": {"]
#[doc = "      \"description\": \"Stable Bun (or external) docs URL for this rule.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"format\": \"uri\""]
#[doc = "    },"]
#[doc = "    \"end_byte\": {"]
#[doc = "      \"description\": \"Byte offset into the scanned file of the finding end (UTF-8).\","]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"line\": {"]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"minimum\": 1.0"]
#[doc = "    },"]
#[doc = "    \"message\": {"]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"original\": {"]
#[doc = "      \"description\": \"Exact text that matched (from source).\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"replacement\": {"]
#[doc = "      \"description\": \"Suggested replacement. Omitted entirely when no replacement is known.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"rule_id\": {"]
#[doc = "      \"description\": \"Rule identifier — slash-separated category/name (e.g. 'api/fs-readFileSync'). Immutable: consumers parse this.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"severity\": {"]
#[doc = "      \"$ref\": \"#/definitions/Severity\""]
#[doc = "    },"]
#[doc = "    \"start_byte\": {"]
#[doc = "      \"description\": \"Byte offset into the scanned file of the finding start (UTF-8).\","]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    #[doc = "True when the rule is only applied by --aggressive. Omitted when false/unset."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub aggressive: ::std::option::Option<bool>,
    #[doc = "True when the rule can be auto-applied by --fix."]
    pub autofix: bool,
    #[doc = "Top-level category derived from rule_id prefix."]
    pub category: ::std::string::String,
    pub col: ::std::num::NonZeroU64,
    #[doc = "Phase 3+ : statut de compat Bun du module hôte. Présent uniquement sur les findings imports/node-* et api/node-*. Optionnel (rétro-compat schema_version=2)."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub compat: ::std::option::Option<Compat>,
    pub confidence: f64,
    pub context: Context,
    #[doc = "Stable Bun (or external) docs URL for this rule."]
    pub docs_url: ::std::string::String,
    #[doc = "Byte offset into the scanned file of the finding end (UTF-8)."]
    pub end_byte: u64,
    pub line: ::std::num::NonZeroU64,
    pub message: ::std::string::String,
    #[doc = "Exact text that matched (from source)."]
    pub original: ::std::string::String,
    #[doc = "Suggested replacement. Omitted entirely when no replacement is known."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub replacement: ::std::option::Option<::std::string::String>,
    #[doc = "Rule identifier — slash-separated category/name (e.g. 'api/fs-readFileSync'). Immutable: consumers parse this."]
    pub rule_id: ::std::string::String,
    pub severity: Severity,
    #[doc = "Byte offset into the scanned file of the finding start (UTF-8)."]
    pub start_byte: u64,
}
impl Finding {
    pub fn builder() -> builder::Finding {
        Default::default()
    }
}
#[doc = "`Mode`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"check\","]
#[doc = "    \"fix\","]
#[doc = "    \"aggressive\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum Mode {
    #[serde(rename = "check")]
    Check,
    #[serde(rename = "fix")]
    Fix,
    #[serde(rename = "aggressive")]
    Aggressive,
}
impl ::std::fmt::Display for Mode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Check => f.write_str("check"),
            Self::Fix => f.write_str("fix"),
            Self::Aggressive => f.write_str("aggressive"),
        }
    }
}
impl ::std::str::FromStr for Mode {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "check" => Ok(Self::Check),
            "fix" => Ok(Self::Fix),
            "aggressive" => Ok(Self::Aggressive),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for Mode {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for Mode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Mode {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "Payload schema for n2b scan results (JSON reports). Mirrors the JSON produced by `n2b --report=json`. JSONL mode wraps each object with a `type` discriminator (\"meta\" for the header, \"finding\" for subsequent lines)."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"$id\": \"https://raw.githubusercontent.com/aphrody-code/n2b/main/schema/v2.json\","]
#[doc = "  \"title\": \"N2bReport\","]
#[doc = "  \"description\": \"Payload schema for n2b scan results (JSON reports). Mirrors the JSON produced by `n2b --report=json`. JSONL mode wraps each object with a `type` discriminator (\\\"meta\\\" for the header, \\\"finding\\\" for subsequent lines).\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"files\","]
#[doc = "    \"files_scanned\","]
#[doc = "    \"findings_total\","]
#[doc = "    \"mode\","]
#[doc = "    \"root\","]
#[doc = "    \"schema_version\","]
#[doc = "    \"tool\","]
#[doc = "    \"version\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"$schema\": {"]
#[doc = "      \"description\": \"URL to this schema.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"format\": \"uri\""]
#[doc = "    },"]
#[doc = "    \"files\": {"]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/FileFix\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"files_scanned\": {"]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"findings_total\": {"]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"mode\": {"]
#[doc = "      \"$ref\": \"#/definitions/Mode\""]
#[doc = "    },"]
#[doc = "    \"root\": {"]
#[doc = "      \"description\": \"Absolute path of the scanned root.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"schema_version\": {"]
#[doc = "      \"description\": \"Schema version, bumped on breaking changes.\","]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"enum\": ["]
#[doc = "        2"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"tool\": {"]
#[doc = "      \"description\": \"Tool name (historically \\\"node2bun\\\").\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"version\": {"]
#[doc = "      \"description\": \"n2b binary semver.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct N2bReport {
    pub files: ::std::vec::Vec<FileFix>,
    pub files_scanned: u64,
    pub findings_total: u64,
    pub mode: Mode,
    #[doc = "Absolute path of the scanned root."]
    pub root: ::std::string::String,
    #[doc = "URL to this schema."]
    #[serde(
        rename = "$schema",
        default,
        skip_serializing_if = "::std::option::Option::is_none"
    )]
    pub schema: ::std::option::Option<::std::string::String>,
    #[doc = "Schema version, bumped on breaking changes."]
    pub schema_version: N2bReportSchemaVersion,
    #[doc = "Tool name (historically \"node2bun\")."]
    pub tool: ::std::string::String,
    #[doc = "n2b binary semver."]
    pub version: ::std::string::String,
}
impl N2bReport {
    pub fn builder() -> builder::N2bReport {
        Default::default()
    }
}
#[doc = "Schema version, bumped on breaking changes."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Schema version, bumped on breaking changes.\","]
#[doc = "  \"type\": \"integer\","]
#[doc = "  \"enum\": ["]
#[doc = "    2"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct N2bReportSchemaVersion(i64);
impl ::std::ops::Deref for N2bReportSchemaVersion {
    type Target = i64;
    fn deref(&self) -> &i64 {
        &self.0
    }
}
impl ::std::convert::From<N2bReportSchemaVersion> for i64 {
    fn from(value: N2bReportSchemaVersion) -> Self {
        value.0
    }
}
impl ::std::convert::TryFrom<i64> for N2bReportSchemaVersion {
    type Error = self::error::ConversionError;
    fn try_from(value: i64) -> ::std::result::Result<Self, self::error::ConversionError> {
        if ![2_i64].contains(&value) {
            Err("invalid value".into())
        } else {
            Ok(Self(value))
        }
    }
}
impl<'de> ::serde::Deserialize<'de> for N2bReportSchemaVersion {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        Self::try_from(<i64>::deserialize(deserializer)?)
            .map_err(|e| <D::Error as ::serde::de::Error>::custom(e.to_string()))
    }
}
#[doc = "`Severity`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"error\","]
#[doc = "    \"warn\","]
#[doc = "    \"info\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum Severity {
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "info")]
    Info,
}
impl ::std::fmt::Display for Severity {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Error => f.write_str("error"),
            Self::Warn => f.write_str("warn"),
            Self::Info => f.write_str("info"),
        }
    }
}
impl ::std::str::FromStr for Severity {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for Severity {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for Severity {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for Severity {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = r" Types for composing complex structures."]
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct Compat {
        bunpp: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        equivalent: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        missing_apis:
            ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        module: ::std::result::Result<::std::string::String, ::std::string::String>,
        status: ::std::result::Result<super::CompatStatus, ::std::string::String>,
    }
    impl ::std::default::Default for Compat {
        fn default() -> Self {
            Self {
                bunpp: Ok(Default::default()),
                equivalent: Ok(Default::default()),
                missing_apis: Ok(Default::default()),
                module: Err("no value supplied for module".to_string()),
                status: Err("no value supplied for status".to_string()),
            }
        }
    }
    impl Compat {
        pub fn bunpp<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.bunpp = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for bunpp: {e}"));
            self
        }
        pub fn equivalent<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.equivalent = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for equivalent: {e}"));
            self
        }
        pub fn missing_apis<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.missing_apis = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for missing_apis: {e}"));
            self
        }
        pub fn module<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.module = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for module: {e}"));
            self
        }
        pub fn status<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::CompatStatus>,
            T::Error: ::std::fmt::Display,
        {
            self.status = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for status: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<Compat> for super::Compat {
        type Error = super::error::ConversionError;
        fn try_from(value: Compat) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                bunpp: value.bunpp?,
                equivalent: value.equivalent?,
                missing_apis: value.missing_apis?,
                module: value.module?,
                status: value.status?,
            })
        }
    }
    impl ::std::convert::From<super::Compat> for Compat {
        fn from(value: super::Compat) -> Self {
            Self {
                bunpp: Ok(value.bunpp),
                equivalent: Ok(value.equivalent),
                missing_apis: Ok(value.missing_apis),
                module: Ok(value.module),
                status: Ok(value.status),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct Context {
        after: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        before:
            ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        line: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for Context {
        fn default() -> Self {
            Self {
                after: Err("no value supplied for after".to_string()),
                before: Err("no value supplied for before".to_string()),
                line: Err("no value supplied for line".to_string()),
            }
        }
    }
    impl Context {
        pub fn after<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.after = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for after: {e}"));
            self
        }
        pub fn before<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.before = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for before: {e}"));
            self
        }
        pub fn line<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.line = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for line: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<Context> for super::Context {
        type Error = super::error::ConversionError;
        fn try_from(value: Context) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                after: value.after?,
                before: value.before?,
                line: value.line?,
            })
        }
    }
    impl ::std::convert::From<super::Context> for Context {
        fn from(value: super::Context) -> Self {
            Self {
                after: Ok(value.after),
                before: Ok(value.before),
                line: Ok(value.line),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct FileFix {
        changed: ::std::result::Result<bool, ::std::string::String>,
        findings: ::std::result::Result<::std::vec::Vec<super::Finding>, ::std::string::String>,
        path: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for FileFix {
        fn default() -> Self {
            Self {
                changed: Err("no value supplied for changed".to_string()),
                findings: Err("no value supplied for findings".to_string()),
                path: Err("no value supplied for path".to_string()),
            }
        }
    }
    impl FileFix {
        pub fn changed<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<bool>,
            T::Error: ::std::fmt::Display,
        {
            self.changed = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for changed: {e}"));
            self
        }
        pub fn findings<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::Finding>>,
            T::Error: ::std::fmt::Display,
        {
            self.findings = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for findings: {e}"));
            self
        }
        pub fn path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for path: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<FileFix> for super::FileFix {
        type Error = super::error::ConversionError;
        fn try_from(value: FileFix) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                changed: value.changed?,
                findings: value.findings?,
                path: value.path?,
            })
        }
    }
    impl ::std::convert::From<super::FileFix> for FileFix {
        fn from(value: super::FileFix) -> Self {
            Self {
                changed: Ok(value.changed),
                findings: Ok(value.findings),
                path: Ok(value.path),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct Finding {
        aggressive: ::std::result::Result<::std::option::Option<bool>, ::std::string::String>,
        autofix: ::std::result::Result<bool, ::std::string::String>,
        category: ::std::result::Result<::std::string::String, ::std::string::String>,
        col: ::std::result::Result<::std::num::NonZeroU64, ::std::string::String>,
        compat: ::std::result::Result<::std::option::Option<super::Compat>, ::std::string::String>,
        confidence: ::std::result::Result<f64, ::std::string::String>,
        context: ::std::result::Result<super::Context, ::std::string::String>,
        docs_url: ::std::result::Result<::std::string::String, ::std::string::String>,
        end_byte: ::std::result::Result<u64, ::std::string::String>,
        line: ::std::result::Result<::std::num::NonZeroU64, ::std::string::String>,
        message: ::std::result::Result<::std::string::String, ::std::string::String>,
        original: ::std::result::Result<::std::string::String, ::std::string::String>,
        replacement: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        rule_id: ::std::result::Result<::std::string::String, ::std::string::String>,
        severity: ::std::result::Result<super::Severity, ::std::string::String>,
        start_byte: ::std::result::Result<u64, ::std::string::String>,
    }
    impl ::std::default::Default for Finding {
        fn default() -> Self {
            Self {
                aggressive: Ok(Default::default()),
                autofix: Err("no value supplied for autofix".to_string()),
                category: Err("no value supplied for category".to_string()),
                col: Err("no value supplied for col".to_string()),
                compat: Ok(Default::default()),
                confidence: Err("no value supplied for confidence".to_string()),
                context: Err("no value supplied for context".to_string()),
                docs_url: Err("no value supplied for docs_url".to_string()),
                end_byte: Err("no value supplied for end_byte".to_string()),
                line: Err("no value supplied for line".to_string()),
                message: Err("no value supplied for message".to_string()),
                original: Err("no value supplied for original".to_string()),
                replacement: Ok(Default::default()),
                rule_id: Err("no value supplied for rule_id".to_string()),
                severity: Err("no value supplied for severity".to_string()),
                start_byte: Err("no value supplied for start_byte".to_string()),
            }
        }
    }
    impl Finding {
        pub fn aggressive<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<bool>>,
            T::Error: ::std::fmt::Display,
        {
            self.aggressive = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for aggressive: {e}"));
            self
        }
        pub fn autofix<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<bool>,
            T::Error: ::std::fmt::Display,
        {
            self.autofix = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for autofix: {e}"));
            self
        }
        pub fn category<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.category = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for category: {e}"));
            self
        }
        pub fn col<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::num::NonZeroU64>,
            T::Error: ::std::fmt::Display,
        {
            self.col = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for col: {e}"));
            self
        }
        pub fn compat<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<super::Compat>>,
            T::Error: ::std::fmt::Display,
        {
            self.compat = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for compat: {e}"));
            self
        }
        pub fn confidence<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<f64>,
            T::Error: ::std::fmt::Display,
        {
            self.confidence = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for confidence: {e}"));
            self
        }
        pub fn context<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::Context>,
            T::Error: ::std::fmt::Display,
        {
            self.context = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for context: {e}"));
            self
        }
        pub fn docs_url<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.docs_url = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for docs_url: {e}"));
            self
        }
        pub fn end_byte<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u64>,
            T::Error: ::std::fmt::Display,
        {
            self.end_byte = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for end_byte: {e}"));
            self
        }
        pub fn line<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::num::NonZeroU64>,
            T::Error: ::std::fmt::Display,
        {
            self.line = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for line: {e}"));
            self
        }
        pub fn message<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.message = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for message: {e}"));
            self
        }
        pub fn original<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.original = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for original: {e}"));
            self
        }
        pub fn replacement<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.replacement = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for replacement: {e}"));
            self
        }
        pub fn rule_id<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.rule_id = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for rule_id: {e}"));
            self
        }
        pub fn severity<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::Severity>,
            T::Error: ::std::fmt::Display,
        {
            self.severity = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for severity: {e}"));
            self
        }
        pub fn start_byte<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u64>,
            T::Error: ::std::fmt::Display,
        {
            self.start_byte = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for start_byte: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<Finding> for super::Finding {
        type Error = super::error::ConversionError;
        fn try_from(value: Finding) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                aggressive: value.aggressive?,
                autofix: value.autofix?,
                category: value.category?,
                col: value.col?,
                compat: value.compat?,
                confidence: value.confidence?,
                context: value.context?,
                docs_url: value.docs_url?,
                end_byte: value.end_byte?,
                line: value.line?,
                message: value.message?,
                original: value.original?,
                replacement: value.replacement?,
                rule_id: value.rule_id?,
                severity: value.severity?,
                start_byte: value.start_byte?,
            })
        }
    }
    impl ::std::convert::From<super::Finding> for Finding {
        fn from(value: super::Finding) -> Self {
            Self {
                aggressive: Ok(value.aggressive),
                autofix: Ok(value.autofix),
                category: Ok(value.category),
                col: Ok(value.col),
                compat: Ok(value.compat),
                confidence: Ok(value.confidence),
                context: Ok(value.context),
                docs_url: Ok(value.docs_url),
                end_byte: Ok(value.end_byte),
                line: Ok(value.line),
                message: Ok(value.message),
                original: Ok(value.original),
                replacement: Ok(value.replacement),
                rule_id: Ok(value.rule_id),
                severity: Ok(value.severity),
                start_byte: Ok(value.start_byte),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct N2bReport {
        files: ::std::result::Result<::std::vec::Vec<super::FileFix>, ::std::string::String>,
        files_scanned: ::std::result::Result<u64, ::std::string::String>,
        findings_total: ::std::result::Result<u64, ::std::string::String>,
        mode: ::std::result::Result<super::Mode, ::std::string::String>,
        root: ::std::result::Result<::std::string::String, ::std::string::String>,
        schema: ::std::result::Result<
            ::std::option::Option<::std::string::String>,
            ::std::string::String,
        >,
        schema_version: ::std::result::Result<super::N2bReportSchemaVersion, ::std::string::String>,
        tool: ::std::result::Result<::std::string::String, ::std::string::String>,
        version: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for N2bReport {
        fn default() -> Self {
            Self {
                files: Err("no value supplied for files".to_string()),
                files_scanned: Err("no value supplied for files_scanned".to_string()),
                findings_total: Err("no value supplied for findings_total".to_string()),
                mode: Err("no value supplied for mode".to_string()),
                root: Err("no value supplied for root".to_string()),
                schema: Ok(Default::default()),
                schema_version: Err("no value supplied for schema_version".to_string()),
                tool: Err("no value supplied for tool".to_string()),
                version: Err("no value supplied for version".to_string()),
            }
        }
    }
    impl N2bReport {
        pub fn files<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::FileFix>>,
            T::Error: ::std::fmt::Display,
        {
            self.files = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for files: {e}"));
            self
        }
        pub fn files_scanned<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u64>,
            T::Error: ::std::fmt::Display,
        {
            self.files_scanned = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for files_scanned: {e}"));
            self
        }
        pub fn findings_total<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u64>,
            T::Error: ::std::fmt::Display,
        {
            self.findings_total = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for findings_total: {e}"));
            self
        }
        pub fn mode<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::Mode>,
            T::Error: ::std::fmt::Display,
        {
            self.mode = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for mode: {e}"));
            self
        }
        pub fn root<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.root = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for root: {e}"));
            self
        }
        pub fn schema<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.schema = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for schema: {e}"));
            self
        }
        pub fn schema_version<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::N2bReportSchemaVersion>,
            T::Error: ::std::fmt::Display,
        {
            self.schema_version = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for schema_version: {e}"));
            self
        }
        pub fn tool<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.tool = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for tool: {e}"));
            self
        }
        pub fn version<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.version = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for version: {e}"));
            self
        }
    }
    impl ::std::convert::TryFrom<N2bReport> for super::N2bReport {
        type Error = super::error::ConversionError;
        fn try_from(
            value: N2bReport,
        ) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                files: value.files?,
                files_scanned: value.files_scanned?,
                findings_total: value.findings_total?,
                mode: value.mode?,
                root: value.root?,
                schema: value.schema?,
                schema_version: value.schema_version?,
                tool: value.tool?,
                version: value.version?,
            })
        }
    }
    impl ::std::convert::From<super::N2bReport> for N2bReport {
        fn from(value: super::N2bReport) -> Self {
            Self {
                files: Ok(value.files),
                files_scanned: Ok(value.files_scanned),
                findings_total: Ok(value.findings_total),
                mode: Ok(value.mode),
                root: Ok(value.root),
                schema: Ok(value.schema),
                schema_version: Ok(value.schema_version),
                tool: Ok(value.tool),
                version: Ok(value.version),
            }
        }
    }
}
