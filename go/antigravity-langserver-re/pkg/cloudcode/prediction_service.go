// SPDX-License-Identifier: Apache-2.0

package cloudcode

// PredictionService is the v1internal generation + quota gRPC service.
//
// reconstructed from: v1internal_prediction_service_go_proto
//
//	(*UnimplementedPredictionServiceServer)<Method>-fm
//
// Method parameter/return types are inferred from the recovered request/response
// message type names (only method *names* are present in the pclntab, not full
// signatures). StreamGenerateContent is server-streaming in the original.
type PredictionService interface {
	// reconstructed from: UnimplementedPredictionServiceServer.GenerateContent-fm
	GenerateContent(*GenerateContentRequest) (*GenerateContentResponse, error)
	// reconstructed from: UnimplementedPredictionServiceServer.StreamGenerateContent-fm
	// Server-streaming in the original; modelled as a chunk channel here.
	StreamGenerateContent(*GenerateContentRequest) (<-chan *GenerateContentResponse, error)
	// reconstructed from: UnimplementedPredictionServiceServer.CountTokens-fm
	CountTokens(*CountTokensRequest) (*CountTokensResponse, error)
	// reconstructed from: UnimplementedPredictionServiceServer.FetchAvailableModels-fm
	FetchAvailableModels(*FetchAvailableModelsRequest) (*FetchAvailableModelsResponse, error)
	// reconstructed from: UnimplementedPredictionServiceServer.RetrieveUserQuota-fm
	RetrieveUserQuota(*RetrieveUserQuotaRequest) (*RetrieveUserQuotaResponse, error)
}

// GenerateContentRequest fields recovered from the proto Get<Field> accessors.
//
// reconstructed from: v1internal_prediction_service_go_proto
//
//	(*GenerateContentRequest)Get{Model,Project,Request,RequestId,RequestType,
//	 UserAgent,UserPromptId,EnabledCreditTypes}
type GenerateContentRequest struct {
	Model              string   // GetModel
	Project            string   // GetProject
	Request            []byte   // GetRequest — opaque inner GenerateContent payload (Vertex/Gemini)
	RequestId          string   // GetRequestId
	RequestType        string   // GetRequestType
	UserAgent          string   // GetUserAgent
	UserPromptId       string   // GetUserPromptId
	EnabledCreditTypes []string // GetEnabledCreditTypes
}

// GenerateContentResponse fields recovered from Get<Field> accessors.
//
// reconstructed from: (*GenerateContentResponse)Get{Response,Metadata,TraceId,
//
//	ConsumedCredits,RemainingCredits}
type GenerateContentResponse struct {
	Response         []byte // GetResponse — opaque inner GenerateContent response
	Metadata         []byte // GetMetadata
	TraceId          string // GetTraceId
	ConsumedCredits  int64  // GetConsumedCredits
	RemainingCredits int64  // GetRemainingCredits
}

// CountTokensRequest. reconstructed from: (*CountTokensRequest)GetRequest
type CountTokensRequest struct {
	Request []byte // GetRequest
}

// CountTokensResponse. reconstructed from: CountTokensResponse type symbol
type CountTokensResponse struct {
	TotalTokens int64
}

// FetchAvailableModelsRequest. reconstructed from: FetchAvailableModelsRequest type symbol
type FetchAvailableModelsRequest struct {
	Project string
}

// FetchAvailableModelsResponse fields recovered from Get<Field> accessors.
//
// reconstructed from: (*FetchAvailableModelsResponse)Get{Models,DefaultAgentModelId,
//
//	AgentModelSorts,BattleModeModelSorts,TabModelIds,CommandModelIds,
//	CommitMessageModelIds,MqueryModelIds,WebSearchModelIds,
//	ImageGenerationModelIds,AudioTranscriptionModelIds,TieredModelIds,
//	DeprecatedModelIds,ExperimentIds}
type FetchAvailableModelsResponse struct {
	Models                     []*ModelDetails // GetModels
	DefaultAgentModelId        string          // GetDefaultAgentModelId
	AgentModelSorts            []*ModelSort    // GetAgentModelSorts
	BattleModeModelSorts       []*ModelSort    // GetBattleModeModelSorts
	TabModelIds                []string        // GetTabModelIds
	CommandModelIds            []string        // GetCommandModelIds
	CommitMessageModelIds      []string        // GetCommitMessageModelIds
	MqueryModelIds             []string        // GetMqueryModelIds
	WebSearchModelIds          []string        // GetWebSearchModelIds
	ImageGenerationModelIds    []string        // GetImageGenerationModelIds
	AudioTranscriptionModelIds []string        // GetAudioTranscriptionModelIds
	TieredModelIds             []string        // GetTieredModelIds
	DeprecatedModelIds         []string        // GetDeprecatedModelIds
	ExperimentIds              []string        // GetExperimentIds
}

// ModelDetails fields recovered from Get<Field> accessors — the per-model
// capability descriptor returned by FetchAvailableModels.
//
// reconstructed from: (*ModelDetails)Get{Model,DisplayName,Description,ApiProvider,
//
//	ModelProvider,VertexModelId,MaxTokens,MaxOutputTokens,SupportsThinking,
//	SupportsRawThinking,SupportsThoughtCirculation,ThinkingBudget,
//	MinThinkingBudget,ThinkingLevel,SupportsImages,SupportsVideo,SupportsPdf,
//	SupportedMimeTypes,SupportsCumulativeContext,SupportsEstimateTokenCounter,
//	Beta,BetaWarningMessage,Preview,Disabled,Recommended,IsInternal,QuotaInfo,
//	ModelExperiments,TokenizerType,ToolFormatterType,ToolResponseKey,
//	PromptTemplaterType,TagTitle,TagDescription,TabJumpPrintLineRange,
//	AddCursorToFindReplaceTarget,RequiresImageOutputOutsideFunctionResponses,
//	RequiresLeadInGeneration,RequiresNoXmlToolExamples}
type ModelDetails struct {
	Model                                       string            // GetModel
	DisplayName                                 string            // GetDisplayName
	Description                                 string            // GetDescription
	ApiProvider                                 string            // GetApiProvider
	ModelProvider                               string            // GetModelProvider
	VertexModelId                               string            // GetVertexModelId
	MaxTokens                                   int64             // GetMaxTokens
	MaxOutputTokens                             int64             // GetMaxOutputTokens
	SupportsThinking                            bool              // GetSupportsThinking
	SupportsRawThinking                         bool              // GetSupportsRawThinking
	SupportsThoughtCirculation                  bool              // GetSupportsThoughtCirculation
	ThinkingBudget                              int64             // GetThinkingBudget
	MinThinkingBudget                           int64             // GetMinThinkingBudget
	ThinkingLevel                               string            // GetThinkingLevel
	SupportsImages                              bool              // GetSupportsImages
	SupportsVideo                               bool              // GetSupportsVideo
	SupportsPdf                                 bool              // GetSupportsPdf
	SupportedMimeTypes                          []string          // GetSupportedMimeTypes
	SupportsCumulativeContext                   bool              // GetSupportsCumulativeContext
	SupportsEstimateTokenCounter                bool              // GetSupportsEstimateTokenCounter
	Beta                                        bool              // GetBeta
	BetaWarningMessage                          string            // GetBetaWarningMessage
	Preview                                     bool              // GetPreview
	Disabled                                    bool              // GetDisabled
	Recommended                                 bool              // GetRecommended
	IsInternal                                  bool              // GetIsInternal
	QuotaInfo                                   *QuotaInfo        // GetQuotaInfo
	ModelExperiments                            *ModelExperiments // GetModelExperiments
	TokenizerType                               string            // GetTokenizerType
	ToolFormatterType                           string            // GetToolFormatterType
	ToolResponseKey                             string            // GetToolResponseKey
	PromptTemplaterType                         string            // GetPromptTemplaterType
	TagTitle                                    string            // GetTagTitle
	TagDescription                              string            // GetTagDescription
	TabJumpPrintLineRange                       int64             // GetTabJumpPrintLineRange
	AddCursorToFindReplaceTarget                bool              // GetAddCursorToFindReplaceTarget
	RequiresImageOutputOutsideFunctionResponses bool              // GetRequiresImageOutputOutsideFunctionResponses
	RequiresLeadInGeneration                    bool              // GetRequiresLeadInGeneration
	RequiresNoXmlToolExamples                   bool              // GetRequiresNoXmlToolExamples
}

// ModelSort. reconstructed from: ModelSort type symbol (FetchAvailableModelsResponse.AgentModelSorts)
type ModelSort struct {
	ModelIds []string
	Label    string
}

// ModelExperiments / ExperimentValue. reconstructed from: ModelExperiments, ExperimentValue type symbols
type ModelExperiments struct {
	Values map[string]*ExperimentValue
}

// ExperimentValue. reconstructed from: ExperimentValue type symbol
type ExperimentValue struct {
	StringValue string
	BoolValue   bool
	IntValue    int64
}

// TieredModelConfig / DeprecatedModelReroutingInfo. reconstructed from: TieredModelConfig,
// DeprecatedModelReroutingInfo type symbols
type TieredModelConfig struct {
	TierModelIds []string
}

type DeprecatedModelReroutingInfo struct {
	FromModelId string
	ToModelId   string
}

// RetrieveUserQuotaRequest. reconstructed from: RetrieveUserQuotaRequest type symbol
type RetrieveUserQuotaRequest struct {
	Project string
}

// RetrieveUserQuotaResponse. reconstructed from: (*RetrieveUserQuotaResponse)GetBuckets
type RetrieveUserQuotaResponse struct {
	Buckets []*QuotaInfo // GetBuckets
}

// QuotaInfo (the "G1 credits" quota bucket).
//
// reconstructed from: (*QuotaInfo)Get{RemainingFraction,ResetTime}
type QuotaInfo struct {
	RemainingFraction float64 // GetRemainingFraction
	ResetTime         int64   // GetResetTime (unix seconds, inferred)
}
