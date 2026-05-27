// SPDX-License-Identifier: Apache-2.0

package langserver

import "context"

// CoreService is a representative, typed subset of LanguageServerService
// covering the highest-value methods (auth, status, the Cascade agent
// lifecycle, file ops). The full 237-method surface is enumerated in Methods
// (methods.go). Request/response types are reconstructed locally because the
// original message types are not encoded in the recovered symbols.
//
// reconstructed from: language_server_go_grpc UnimplementedLanguageServerServiceServer
type CoreService interface {
	// Lifecycle / health.
	Heartbeat(ctx context.Context, req *HeartbeatRequest) (*HeartbeatResponse, error) // Heartbeat
	GetStatus(ctx context.Context, req *GetStatusRequest) (*GetStatusResponse, error) // GetStatus
	Exit(ctx context.Context) error                                                   // Exit
	Restart(ctx context.Context) error                                                // Restart

	// Auth.
	GetAuthStatus(ctx context.Context) (*AuthStatusResponse, error) // GetAuthStatus
	LoginWithBrowser(ctx context.Context) error                     // LoginWithBrowser
	AuthLogout(ctx context.Context) error                           // AuthLogout
	HasAuthToken(ctx context.Context) (bool, error)                 // HasAuthToken
	GetGrantedScopes(ctx context.Context) ([]string, error)         // GetGrantedScopes
	FetchUserInfo(ctx context.Context) (*UserInfoResponse, error)   // FetchUserInfo

	// Cascade agent lifecycle (delegates to cortex.CascadeManager).
	StartCascade(ctx context.Context, req *StartCascadeRequest) (*StartCascadeResponse, error) // StartCascade
	SendUserCascadeMessage(ctx context.Context, req *SendUserCascadeMessageRequest) error      // SendUserCascadeMessage
	CancelCascadeInvocation(ctx context.Context, conversationID string) error                  // CancelCascadeInvocation
	ForceStopCascadeTree(ctx context.Context, conversationID string) error                     // ForceStopCascadeTree
	RevertToCascadeStep(ctx context.Context, req *RevertToCascadeStepRequest) error            // RevertToCascadeStep

	// BattleMode (side-by-side model comparison).
	StartBattleMode(ctx context.Context, modelIDs []string) error // StartBattleMode
	EndBattleMode(ctx context.Context) error                      // EndBattleMode

	// File operations (the agent's local FS surface).
	ReadFile(ctx context.Context, uri string) ([]byte, error)        // ReadFile
	WriteFile(ctx context.Context, uri string, data []byte) error    // WriteFile
	ReadDir(ctx context.Context, uri string) ([]string, error)       // ReadDir
	DeleteFileOrDirectory(ctx context.Context, uri string) error     // DeleteFileOrDirectory
	SearchCode(ctx context.Context, query string) ([]string, error)  // SearchCode
	SearchFiles(ctx context.Context, query string) ([]string, error) // SearchFiles
}

// HeartbeatRequest / HeartbeatResponse. reconstructed from: Heartbeat-fm
type HeartbeatRequest struct{ ProcessID int64 }
type HeartbeatResponse struct{ Healthy bool }

// GetStatusRequest / GetStatusResponse. reconstructed from: GetStatus-fm
type GetStatusRequest struct{}
type GetStatusResponse struct {
	Version       string
	Ready         bool
	ActiveProject string
}

// AuthStatusResponse. reconstructed from: GetAuthStatus-fm
type AuthStatusResponse struct {
	LoggedIn bool
	Email    string
	Scopes   []string
}

// UserInfoResponse. reconstructed from: FetchUserInfo-fm
type UserInfoResponse struct {
	Email      string
	RegionCode string
}

// StartCascadeRequest / StartCascadeResponse. reconstructed from: StartCascade-fm
type StartCascadeRequest struct {
	ConversationID string
	ModelID        string
	InitialMessage string
}
type StartCascadeResponse struct {
	ConversationID string
}

// SendUserCascadeMessageRequest. reconstructed from: SendUserCascadeMessage-fm
type SendUserCascadeMessageRequest struct {
	ConversationID string
	Message        string
}

// RevertToCascadeStepRequest. reconstructed from: RevertToCascadeStep-fm
type RevertToCascadeStepRequest struct {
	ConversationID string
	StepIndex      int32
}
