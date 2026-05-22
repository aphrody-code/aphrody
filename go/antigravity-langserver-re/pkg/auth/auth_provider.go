// SPDX-License-Identifier: Apache-2.0

package auth

import "net/http"

// AuthProvider is the common interface implemented by all four auth providers
// in code_assist_client/auth_provider.go. The method set is the union of the
// methods recovered on every implementation (IDE/Standalone/CLI/Hub all share
// GetTokenSource, GetCAICProject, GetGCPLocation, IsBusinessPayGo,
// GetHTTPClient, GetUserAgentName, GetEndpointURL, SetEndpointURL, IsEval,
// ShouldUseCAICProjectAsOverride, SetHTTPHeaders).
//
// reconstructed from: codeassistclient/auth_provider.go method sets of
//
//	IDEAuthProvider, StandaloneAuthProvider, CLIAuthProvider, AntigravityHubAuthProvider
type AuthProvider interface {
	GetTokenSource() (TokenSource, error)   // GetTokenSource
	GetCAICProject() string                 // GetCAICProject (CloudAICompanion project)
	GetGCPLocation() string                 // GetGCPLocation
	IsBusinessPayGo() bool                  // IsBusinessPayGo
	GetHTTPClient() *http.Client            // GetHTTPClient
	GetUserAgentName() string               // GetUserAgentName
	GetEndpointURL() string                 // GetEndpointURL
	SetEndpointURL(url string)              // SetEndpointURL
	IsEval() bool                           // IsEval
	ShouldUseCAICProjectAsOverride() bool   // ShouldUseCAICProjectAsOverride
	SetHTTPHeaders(req *http.Request) error // SetHTTPHeaders
}

// TokenSource is a minimal recovered token source abstraction (golang.org/x/oauth2
// shape; we model it locally to avoid a runtime dependency in the RE module).
type TokenSource interface {
	Token() (accessToken string, err error)
}

// IDEAuthProvider — token source backed by the IDE's credential store; the
// richest implementation (85-line SetHTTPHeaders).
//
// reconstructed from: (*IDEAuthProvider) method set in auth_provider.go
type IDEAuthProvider struct {
	endpointURL string
	caicProject string
	gcpLocation string
}

// StandaloneAuthProvider — for the standalone (non-IDE) language server mode.
//
// reconstructed from: (*StandaloneAuthProvider) method set
type StandaloneAuthProvider struct {
	endpointURL string
}

// CLIAuthProvider — for the jetski CLI; exposes setters for token source,
// quota project, GCP location and business-auth fetchers.
//
// reconstructed from: (*CLIAuthProvider) method set incl. Set{TokenSource,
//
//	QuotaProject,QuotaProjectFetcher,GCPLocationFetcher,IsBusinessAuthFetcher}
type CLIAuthProvider struct {
	endpointURL  string
	tokenSource  TokenSource
	quotaProject string
	gcpLocation  string
}

// AntigravityHubAuthProvider — hub/enterprise variant; carries explicit
// project/location and an OAuth token-info accessor.
//
// reconstructed from: (*AntigravityHubAuthProvider) method set incl.
//
//	Set{ProjectID,Location,TokenInfo}, OAuthTokenInfo
type AntigravityHubAuthProvider struct {
	projectID string
	location  string
}

// Compile-time assertions: every reconstructed provider satisfies AuthProvider.
var (
	_ AuthProvider = (*IDEAuthProvider)(nil)
	_ AuthProvider = (*StandaloneAuthProvider)(nil)
	_ AuthProvider = (*CLIAuthProvider)(nil)
	_ AuthProvider = (*AntigravityHubAuthProvider)(nil)
)

func (p *IDEAuthProvider) GetTokenSource() (TokenSource, error)   { return nil, nil }
func (p *IDEAuthProvider) GetCAICProject() string                 { return p.caicProject }
func (p *IDEAuthProvider) GetGCPLocation() string                 { return p.gcpLocation }
func (p *IDEAuthProvider) IsBusinessPayGo() bool                  { return false }
func (p *IDEAuthProvider) GetHTTPClient() *http.Client            { return http.DefaultClient }
func (p *IDEAuthProvider) GetUserAgentName() string               { return "antigravity-language-server" }
func (p *IDEAuthProvider) GetEndpointURL() string                 { return p.endpointURL }
func (p *IDEAuthProvider) SetEndpointURL(url string)              { p.endpointURL = url }
func (p *IDEAuthProvider) IsEval() bool                           { return false }
func (p *IDEAuthProvider) ShouldUseCAICProjectAsOverride() bool   { return false }
func (p *IDEAuthProvider) SetHTTPHeaders(req *http.Request) error { _ = req; return nil }

func (p *StandaloneAuthProvider) GetTokenSource() (TokenSource, error)   { return nil, nil }
func (p *StandaloneAuthProvider) GetCAICProject() string                 { return "" }
func (p *StandaloneAuthProvider) GetGCPLocation() string                 { return "" }
func (p *StandaloneAuthProvider) IsBusinessPayGo() bool                  { return false }
func (p *StandaloneAuthProvider) GetHTTPClient() *http.Client            { return http.DefaultClient }
func (p *StandaloneAuthProvider) GetUserAgentName() string               { return "antigravity-language-server" }
func (p *StandaloneAuthProvider) GetEndpointURL() string                 { return p.endpointURL }
func (p *StandaloneAuthProvider) SetEndpointURL(url string)              { p.endpointURL = url }
func (p *StandaloneAuthProvider) IsEval() bool                           { return false }
func (p *StandaloneAuthProvider) ShouldUseCAICProjectAsOverride() bool   { return false }
func (p *StandaloneAuthProvider) SetHTTPHeaders(req *http.Request) error { _ = req; return nil }

func (p *CLIAuthProvider) SetTokenSource(ts TokenSource)          { p.tokenSource = ts }
func (p *CLIAuthProvider) SetQuotaProject(project string)         { p.quotaProject = project }
func (p *CLIAuthProvider) GetTokenSource() (TokenSource, error)   { return p.tokenSource, nil }
func (p *CLIAuthProvider) GetCAICProject() string                 { return p.quotaProject }
func (p *CLIAuthProvider) GetGCPLocation() string                 { return p.gcpLocation }
func (p *CLIAuthProvider) IsBusinessPayGo() bool                  { return false }
func (p *CLIAuthProvider) GetHTTPClient() *http.Client            { return http.DefaultClient }
func (p *CLIAuthProvider) GetUserAgentName() string               { return "jetski-cli" }
func (p *CLIAuthProvider) GetEndpointURL() string                 { return p.endpointURL }
func (p *CLIAuthProvider) SetEndpointURL(url string)              { p.endpointURL = url }
func (p *CLIAuthProvider) IsEval() bool                           { return false }
func (p *CLIAuthProvider) ShouldUseCAICProjectAsOverride() bool   { return false }
func (p *CLIAuthProvider) SetHTTPHeaders(req *http.Request) error { _ = req; return nil }

func (p *AntigravityHubAuthProvider) SetProjectID(id string)                 { p.projectID = id }
func (p *AntigravityHubAuthProvider) SetLocation(loc string)                 { p.location = loc }
func (p *AntigravityHubAuthProvider) GetTokenSource() (TokenSource, error)   { return nil, nil }
func (p *AntigravityHubAuthProvider) OAuthTokenInfo() string                 { return "" }
func (p *AntigravityHubAuthProvider) GetCAICProject() string                 { return p.projectID }
func (p *AntigravityHubAuthProvider) GetGCPLocation() string                 { return p.location }
func (p *AntigravityHubAuthProvider) IsBusinessPayGo() bool                  { return true }
func (p *AntigravityHubAuthProvider) GetHTTPClient() *http.Client            { return http.DefaultClient }
func (p *AntigravityHubAuthProvider) GetUserAgentName() string               { return "antigravity-hub" }
func (p *AntigravityHubAuthProvider) GetEndpointURL() string                 { return "" }
func (p *AntigravityHubAuthProvider) SetEndpointURL(url string)              { _ = url }
func (p *AntigravityHubAuthProvider) IsEval() bool                           { return false }
func (p *AntigravityHubAuthProvider) ShouldUseCAICProjectAsOverride() bool   { return true }
func (p *AntigravityHubAuthProvider) SetHTTPHeaders(req *http.Request) error { _ = req; return nil }
