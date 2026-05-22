// SPDX-License-Identifier: Apache-2.0

package cloudcode

// JetskiService is the v1internal "Jetski" gRPC service: account/info, agent
// plugins, telemetry and URL/denylist helpers.
//
// reconstructed from: v1internal_jetski_service_go_proto
//
//	(*UnimplementedJetskiServiceServer)<Method>-fm
type JetskiService interface {
	FetchUserInfo(*FetchUserInfoRequest) (*FetchUserInfoResponse, error)                                        // FetchUserInfo-fm
	SetUserSettings(*SetUserSettingsRequest) (*SetUserSettingsResponse, error)                                  // SetUserSettings-fm
	TabChat(*TabChatRequest) (*TabChatResponse, error)                                                          // TabChat-fm
	ListAgentPlugins(*ListAgentPluginsRequest) (*ListAgentPluginsResponse, error)                               // ListAgentPlugins-fm
	GetAgentPlugin(*GetAgentPluginRequest) (*AgentPlugin, error)                                                // GetAgentPlugin-fm
	ListBuildWithGooglePlugins(*ListBuildWithGooglePluginsRequest) (*ListBuildWithGooglePluginsResponse, error) // ListBuildWithGooglePlugins-fm
	ListCascadeNuxes(*ListCascadeNuxesRequest) (*ListCascadeNuxesResponse, error)                               // ListCascadeNuxes-fm
	ListWebDocsOptions(*ListWebDocsOptionsRequest) (*ListWebDocsOptionsResponse, error)                         // ListWebDocsOptions-fm
	RewriteUri(*RewriteUriRequest) (*RewriteUriResponse, error)                                                 // RewriteUri-fm
	CheckUrlDenylist(*CheckUrlDenylistRequest) (*CheckUrlDenylistResponse, error)                               // CheckUrlDenylist-fm
	FetchFromTrawlerCache(*FetchFromTrawlerCacheRequest) (*FetchFromTrawlerCacheResponse, error)                // FetchFromTrawlerCache-fm
	RecordTrajectoryAnalytics(*RecordTrajectoryAnalyticsRequest) error                                          // RecordTrajectoryAnalytics-fm
	RegisterInteraction(*RegisterInteractionRequest) (*RegisterInteractionResponse, error)                      // RegisterInteraction-fm
	BattleModeOverrides(*BattleModeOverridesRequest) (*BattleModeOverridesResponse, error)                      // BattleModeOverrides-fm
	GetHealth(*GetHealthRequest) (*Health, error)                                                               // GetHealth-fm
}

// NuxClient is a recovered enum on the JetskiService proto.
//
// reconstructed from: (*NuxClient)Enum / EnumDescriptor / Number / String
type NuxClient int32

// FetchUserInfoRequest/Response. reconstructed from: (*FetchUserInfoRequest)GetProject,
// (*FetchUserInfoResponse)Get{UserSettings,UserTags,RegionCode}
type FetchUserInfoRequest struct {
	Project string // GetProject
}

type FetchUserInfoResponse struct {
	UserSettings *UserSettings // GetUserSettings
	UserTags     []string      // GetUserTags
	RegionCode   string        // GetRegionCode
}

// UserSettings. reconstructed from: (*UserSettings)Get{TelemetryEnabled,
// MarketingEmailsEnabled,UserDataCollectionForceDisabled}
type UserSettings struct {
	TelemetryEnabled                bool // GetTelemetryEnabled
	MarketingEmailsEnabled          bool // GetMarketingEmailsEnabled
	UserDataCollectionForceDisabled bool // GetUserDataCollectionForceDisabled
}

// SetUserSettingsRequest/Response. reconstructed from: (*SetUserSettingsRequest)GetUserSettings,
// (*SetUserSettingsResponse)GetUserSettings
type SetUserSettingsRequest struct {
	UserSettings *UserSettings // GetUserSettings
}

type SetUserSettingsResponse struct {
	UserSettings *UserSettings // GetUserSettings
}

// TabChatRequest/Response. reconstructed from: (*TabChatRequest)Get{Project,Request},
// (*TabChatResponse)GetResponse
type TabChatRequest struct {
	Project string // GetProject
	Request []byte // GetRequest
}

type TabChatResponse struct {
	Response []byte // GetResponse
}

// AgentPlugin and its config tree.
//
// reconstructed from: (*AgentPlugin)Get{Uid,Name,Description,Readme,Link,
//
//	InstallationCount,TrustLevel,Local,Remote,Configuration} and the
//	AgentPlugin{Local,Remote}Config / AgentPluginCommand* accessor sets.
type AgentPlugin struct {
	Uid               string                   // GetUid
	Name              string                   // GetName
	Description       string                   // GetDescription
	Readme            string                   // GetReadme
	Link              string                   // GetLink
	InstallationCount int64                    // GetInstallationCount
	TrustLevel        string                   // GetTrustLevel
	Local             *AgentPluginLocalConfig  // GetLocal
	Remote            *AgentPluginRemoteConfig // GetRemote
	Configuration     []byte                   // GetConfiguration
}

// reconstructed from: (*AgentPluginLocalConfig)GetCommands
type AgentPluginLocalConfig struct {
	Commands []*AgentPluginCommand // GetCommands
}

// reconstructed from: (*AgentPluginRemoteConfig)GetRemoteTemplate
type AgentPluginRemoteConfig struct {
	RemoteTemplate *AgentPluginRemoteConfigTemplate // GetRemoteTemplate
}

// reconstructed from: (*AgentPluginRemoteConfigTemplate)Get{ServerUrl,AuthProviderType,Headers}
type AgentPluginRemoteConfigTemplate struct {
	ServerUrl        string            // GetServerUrl
	AuthProviderType string            // GetAuthProviderType
	Headers          map[string]string // GetHeaders
}

// reconstructed from: (*AgentPluginCommand)Get{CommandTemplate,Variables}
type AgentPluginCommand struct {
	CommandTemplate *AgentPluginCommandTemplate   // GetCommandTemplate
	Variables       []*AgentPluginCommandVariable // GetVariables
}

// reconstructed from: (*AgentPluginCommandTemplate)Get{Command,Args,Env}
type AgentPluginCommandTemplate struct {
	Command string            // GetCommand
	Args    []string          // GetArgs
	Env     map[string]string // GetEnv
}

// reconstructed from: (*AgentPluginCommandVariable)Get{Name,Title,Description,Link}
type AgentPluginCommandVariable struct {
	Name        string // GetName
	Title       string // GetTitle
	Description string // GetDescription
	Link        string // GetLink
}

// ListAgentPluginsRequest/Response. reconstructed from: (*ListAgentPluginsRequest)Get{Filter,
// PageSize,PageToken}, (*ListAgentPluginsResponse)Get{AgentPlugins,NextPageToken}
type ListAgentPluginsRequest struct {
	Filter    string // GetFilter
	PageSize  int32  // GetPageSize
	PageToken string // GetPageToken
}

type ListAgentPluginsResponse struct {
	AgentPlugins  []*AgentPlugin // GetAgentPlugins
	NextPageToken string         // GetNextPageToken
}

// GetAgentPluginRequest. reconstructed from: (*GetAgentPluginRequest)GetName
type GetAgentPluginRequest struct {
	Name string // GetName
}

// BuildWithGooglePlugin. reconstructed from: (*BuildWithGooglePlugin)Get{Plugin,Source,
// Gstatic,VersionShas} and GStaticSource.GetLink
type BuildWithGooglePlugin struct {
	Plugin      *AgentPlugin   // GetPlugin
	Source      string         // GetSource
	Gstatic     *GStaticSource // GetGstatic
	VersionShas []string       // GetVersionShas
}

// reconstructed from: (*GStaticSource)GetLink
type GStaticSource struct {
	Link string // GetLink
}

type ListBuildWithGooglePluginsRequest struct{}

type ListBuildWithGooglePluginsResponse struct {
	Plugins []*BuildWithGooglePlugin
}

// CascadeNux — onboarding "new user experience" entry.
//
// reconstructed from: (*CascadeNux)Get{Uid,Title,Body,Icon,ImageUrl,VideoUrl,
//
//	Location,Trigger,Filter,Priority,AnalyticsEvent,AvailableInteractions,
//	LearnMoreUri,LearnMoreButtonText,PrimaryCtaText,RequiresIdleCascade}
type CascadeNux struct {
	Uid                   string   // GetUid
	Title                 string   // GetTitle
	Body                  string   // GetBody
	Icon                  string   // GetIcon
	ImageUrl              string   // GetImageUrl
	VideoUrl              string   // GetVideoUrl
	Location              string   // GetLocation
	Trigger               string   // GetTrigger
	Filter                string   // GetFilter
	Priority              int32    // GetPriority
	AnalyticsEvent        string   // GetAnalyticsEvent
	AvailableInteractions []string // GetAvailableInteractions
	LearnMoreUri          string   // GetLearnMoreUri
	LearnMoreButtonText   string   // GetLearnMoreButtonText
	PrimaryCtaText        string   // GetPrimaryCtaText
	RequiresIdleCascade   bool     // GetRequiresIdleCascade
}

type ListCascadeNuxesRequest struct{}

type ListCascadeNuxesResponse struct {
	Nuxes []*CascadeNux
}

type ListWebDocsOptionsRequest struct{}

type ListWebDocsOptionsResponse struct {
	Options []string
}

// RewriteUriRequest/Response. reconstructed from: (*RewriteUriRequest)GetOriginalUri,
// (*RewriteUriResponse)GetRedirectUri
type RewriteUriRequest struct {
	OriginalUri string // GetOriginalUri
}

type RewriteUriResponse struct {
	RedirectUri string // GetRedirectUri
}

// CheckUrlDenylistRequest/Response. reconstructed from: (*CheckUrlDenylistRequest)GetUrl,
// (*CheckUrlDenylistResponse)GetIsDenied
type CheckUrlDenylistRequest struct {
	Url string // GetUrl
}

type CheckUrlDenylistResponse struct {
	IsDenied bool // GetIsDenied
}

// FetchFromTrawlerCacheRequest/Response — the "Trawler" web cache.
//
// reconstructed from: (*FetchFromTrawlerCacheRequest)Get{Url,LiveFetch},
// (*FetchFromTrawlerCacheResponse)GetContent
type FetchFromTrawlerCacheRequest struct {
	Url       string // GetUrl
	LiveFetch bool   // GetLiveFetch
}

type FetchFromTrawlerCacheResponse struct {
	Content string // GetContent
}

// RecordTrajectoryAnalyticsRequest. reconstructed from: (*RecordTrajectoryAnalyticsRequest)Get{
// Trajectory,Metadata,MendelExperimentIds,StartStepIndex,StartGeneratorMetadataIndex}
type RecordTrajectoryAnalyticsRequest struct {
	Trajectory                  []byte   // GetTrajectory
	Metadata                    []byte   // GetMetadata
	MendelExperimentIds         []string // GetMendelExperimentIds
	StartStepIndex              int32    // GetStartStepIndex
	StartGeneratorMetadataIndex int32    // GetStartGeneratorMetadataIndex
}

// RegisterInteractionRequest/Response. reconstructed from: (*RegisterInteractionRequest)GetInteraction,
// (*RegisterInteractionResponse)GetMessage
type RegisterInteractionRequest struct {
	Interaction []byte // GetInteraction
}

type RegisterInteractionResponse struct {
	Message string // GetMessage
}

// BattleModeOverridesRequest/Response. reconstructed from: (*BattleModeOverridesRequest)GetModelIds,
// (*BattleModeOverridesResponse)Get{ShouldOverride,OverrideModelIds}
type BattleModeOverridesRequest struct {
	ModelIds []string // GetModelIds
}

type BattleModeOverridesResponse struct {
	ShouldOverride   bool     // GetShouldOverride
	OverrideModelIds []string // GetOverrideModelIds
}

// GetHealthRequest / Health. reconstructed from: (*GetHealthRequest)GetName, (*Health)GetName
type GetHealthRequest struct {
	Name string // GetName
}

type Health struct {
	Name string // GetName
}
