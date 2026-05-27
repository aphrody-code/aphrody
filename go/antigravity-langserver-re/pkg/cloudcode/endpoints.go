// SPDX-License-Identifier: Apache-2.0

package cloudcode

// Backend hosts.
//
// reconstructed from: ls-strings.txt (https://...googleapis.com literals)
const (
	HostCloudCodePA      = "https://cloudcode-pa.googleapis.com"
	HostDailyCloudCodePA = "https://daily-cloudcode-pa.googleapis.com"
	HostAIPlatform       = "https://aiplatform.googleapis.com"
	HostAICodeGRPC       = "aicode.googleapis.com:443"

	// Telemetry / feedback / security surfaces also referenced by the LS.
	HostFeedbackPA     = "https://feedback-pa.googleapis.com"
	HostPlayLog        = "https://play.googleapis.com/log"
	HostDocs           = "https://docs.googleapis.com"
	HostIAMCredentials = "https://iamcredentials.googleapis.com"
	HostAlkaliApplets  = "https://alkalimakersuiteapplets.pa.googleapis.com"
)

// REST method prefix. The gRPC JetskiService/PredictionService are transcoded
// to REST as v1internal:<method>.
//
// reconstructed from: var/data/antigravity-ide-re/v1internal-methods.txt
const RESTMethodPrefix = "v1internal:"

// V1InternalMethods is the full set of v1internal:* REST methods recovered from
// the binary (37). Methods that map to a recovered gRPC service method are
// annotated below in jetski_service.go / prediction_service.go.
//
// reconstructed from: var/data/antigravity-ide-re/v1internal-methods.txt
var V1InternalMethods = []string{
	"battleModeOverrides",
	"checkUrlDenylist",
	"completeCode",
	"countTokens",
	"fetchAdminControls",
	"fetchAvailableModels",
	"fetchCodeCustomizationState",
	"fetchFromTrawlerCache",
	"fetchUserInfo",
	"generateChat",
	"generateCode",
	"generateContent",
	"getCodeAssistGlobalUserSetting",
	"internalAtomicAgenticChat",
	"listAgents",
	"listCloudAICompanionProjects",
	"listExperiments",
	"listModelConfigs",
	"listRemoteRepositories",
	"loadCodeAssist",
	"migrateDatabaseCode",
	"onboardUser",
	"onboardUserBackgroundTasks",
	"recordClientEvent",
	"recordCodeAssistMetrics",
	"recordSmartchoicesFeedback",
	"recordTrajectoryAnalytics",
	"registerInteraction",
	"retrieveUserQuota",
	"rewriteUri",
	"searchSnippets",
	"setCodeAssistGlobalUserSetting",
	"setUserSettings",
	"streamGenerateChat",
	"streamGenerateContent",
	"tabChat",
	"transformCode",
}
