// SPDX-License-Identifier: Apache-2.0

package auth

import "context"

// AuthStatus is the auth state surfaced by AuthClient.GetAuthStatus.
//
// reconstructed from: (*AuthClient)GetAuthStatus / validateLoginAndUpdateStatus
type AuthStatus int32

const (
	AuthStatusUnknown AuthStatus = iota
	AuthStatusLoggedOut
	AuthStatusLoggedIn
	AuthStatusInvalid
)

// AuthClient is the language server's OAuth login + validation client.
//
// reconstructed from: third_party/jetski/language_server/auth_client/authclient
//
//	(auth_client.go) — method set recovered from redress source projection.
//	Method bodies (e.g. the 433-line LoginWithBrowser, the 498-line
//	performTerminalAuthFlow) are NOT recoverable from the stripped binary; the
//	interface and behaviour are documented only.
type AuthClient interface {
	// GetAuthStatus returns the current cached auth status.
	// reconstructed from: (*AuthClient)GetAuthStatus
	GetAuthStatus(ctx context.Context) (AuthStatus, error)
	// LoginWithBrowser runs the loopback PKCE browser flow (433-line original).
	// reconstructed from: (*AuthClient)LoginWithBrowser
	LoginWithBrowser(ctx context.Context) error
	// Logout clears the persisted token.
	// reconstructed from: (*AuthClient)Logout
	Logout(ctx context.Context) error
	// HasAuthToken reports whether a token is present in the credential store.
	// reconstructed from: (*AuthClient)HasAuthToken
	HasAuthToken() bool
	// ValidateProject validates the active CAIC project for the token.
	// reconstructed from: (*AuthClient)ValidateProject
	ValidateProject(ctx context.Context, project string) error
	// GetGrantedScopes returns the scopes actually granted to the token.
	// reconstructed from: (*AuthClient)GetGrantedScopes
	GetGrantedScopes(ctx context.Context) ([]string, error)
}

// validationKind mirrors the three private validation paths recovered in
// auth_client.go: internalValidation, personalValidation, enterpriseValidation.
//
// reconstructed from: (*AuthClient){internalValidation,personalValidation,enterpriseValidation}
type validationKind int

const (
	validationInternal   validationKind = iota // internalValidation
	validationPersonal                         // personalValidation
	validationEnterprise                       // enterpriseValidation
)

// generateState mirrors the recovered free function used to build the OAuth
// state parameter for the loopback flow.
//
// reconstructed from: authclient.generateState
func generateState() string { return "" }

// openBrowser mirrors the recovered free function that launches the system
// browser for the login URL.
//
// reconstructed from: authclient.openBrowser
func openBrowser(url string) error { _ = url; return nil }

// keep the recovered private symbols referenced so vet/build retain them as
// documented reconstruction surface.
var _ = []validationKind{validationInternal, validationPersonal, validationEnterprise}
var _ = generateState
var _ = openBrowser
