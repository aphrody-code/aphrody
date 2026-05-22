// SPDX-License-Identifier: Apache-2.0

package auth

// OAuth desktop client IDs embedded in the binary (public by design — desktop
// Auth-Code + PKCE, no confidential client secret). Recovered byte-for-byte.
//
// reconstructed from: ls-strings.txt (*.apps.googleusercontent.com)
const (
	ClientIDPrimary   = "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com"
	ClientIDSecondary = "884354919052-36trc1jjb3tguiac32ov6cod268c5blh.apps.googleusercontent.com"
)

// OAuth 2.0 endpoints.
//
// reconstructed from: ls-strings.txt
const (
	EndpointAuth       = "https://accounts.google.com/o/oauth2/v2/auth"
	EndpointToken      = "https://oauth2.googleapis.com/token"
	EndpointTokenMTLS  = "https://oauth2.mtls.googleapis.com/token"
	EndpointDeviceCode = "https://oauth2.googleapis.com/device/code" // headless device-code flow
	EndpointUserInfo   = "https://www.googleapis.com/oauth2/v2/userinfo"
	EndpointTokenInfo  = "https://www.googleapis.com/oauth2/v3/tokeninfo"
)

// OAuth scopes requested by the language server.
//
// reconstructed from: ls-strings.txt (https://www.googleapis.com/auth/*)
var Scopes = []string{
	"https://www.googleapis.com/auth/cloud-platform",
	"https://www.googleapis.com/auth/userinfo.email",
	"https://www.googleapis.com/auth/userinfo.profile",
	"https://www.googleapis.com/auth/cclog",
	"https://www.googleapis.com/auth/experimentsandconfigs",
	"https://www.googleapis.com/auth/aicode",
	// Drive family (sidecars + Drive read access for the agent).
	"https://www.googleapis.com/auth/drive",
	"https://www.googleapis.com/auth/drive.appdata",
	"https://www.googleapis.com/auth/drive.file",
	"https://www.googleapis.com/auth/drive.metadata",
	"https://www.googleapis.com/auth/drive.metadata.readonly",
	"https://www.googleapis.com/auth/drive.apps.readonly",
	"https://www.googleapis.com/auth/drive.photos.readonly",
}

// CredentialKey is the OS credential-store key the token is persisted under.
//
// reconstructed from: docs/research/antigravity-ide-re.md §5.5 + ls-strings
const CredentialKey = "gemini:antigravity"
