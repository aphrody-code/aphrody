// SPDX-License-Identifier: Apache-2.0

package cascade

import (
	"context"

	"github.com/aphrody-code/aphrody/go/antigravity-langserver-re/pkg/cortex"
)

// Manager is the reconstructed CascadeManager interface — the agent run loop.
// Every method maps to a recovered (*CascadeManager)<Method> symbol from
// cascade_manager.go / battlemode.go. Signatures are reconstructed (only the
// names are recoverable); types reference the cortex trajectory model.
//
// reconstructed from: cortex/cortex cascade_manager.go + battlemode.go
type Manager interface {
	// Run lifecycle.
	WarmUpAgentProcess(ctx context.Context, conversationID string) error               // WarmUpAgentProcess
	MaybeReviveAgent(ctx context.Context, conversationID string) error                 // MaybeReviveAgent
	IsCascadeRunning(conversationID string) bool                                       // IsCascadeRunning
	ExecutionStatus(conversationID string) ExecutionStatus                             // ExecutionStatus
	Cancel(ctx context.Context, conversationID string) error                           // Cancel
	CancelSteps(ctx context.Context, conversationID string, stepIndices []int32) error // CancelSteps
	ForceStopTree(ctx context.Context, conversationID string) error                    // ForceStopTree
	Shutdown(ctx context.Context) error                                                // Shutdown
	SignalExecutableIdle(conversationID string)                                        // SignalExecutableIdle
	WaitForConversationFullyIdle(ctx context.Context, conversationID string) error     // WaitForConversationFullyIdle

	// Messaging / interaction.
	SendUserCascadeMessage(ctx context.Context, conversationID, message string) error            // SendUserCascadeMessage
	SendAllQueuedMessages(ctx context.Context, conversationID string) error                      // SendAllQueuedMessages
	DeleteQueuedUserInputStep(ctx context.Context, conversationID string, stepIndex int32) error // DeleteQueuedUserInputStep
	HandleUserInteraction(ctx context.Context, conversationID string, payload []byte) error      // HandleUserInteraction
	SendStepsToBackground(ctx context.Context, conversationID string, stepIndices []int32) error // SendStepsToBackground
	SkipBrowserSubagent(ctx context.Context, conversationID string) error                        // SkipBrowserSubagent
	KillSubagent(ctx context.Context, conversationID string) error                               // KillSubagent

	// Trajectory management.
	GetTrajectory(ctx context.Context, id string) (*cortex.Trajectory, error)                             // GetTrajectory
	GetAllTrajectories(ctx context.Context) ([]*cortex.Trajectory, error)                                 // GetAllTrajectories
	GetMainAgentTrajectories(ctx context.Context) ([]*cortex.Trajectory, error)                           // GetMainAgentTrajectories
	LoadTrajectory(ctx context.Context, id string) (*cortex.Trajectory, error)                            // LoadTrajectory
	ReloadTrajectoryIfLoaded(ctx context.Context, id string) error                                        // ReloadTrajectoryIfLoaded
	SaveTrajectory(ctx context.Context, t *cortex.Trajectory) error                                       // SaveTrajectory
	DeleteTrajectory(ctx context.Context, id string) error                                                // DeleteTrajectory
	RevertToStep(ctx context.Context, conversationID string, stepIndex int32) error                       // RevertToStep
	GetRevertPreview(ctx context.Context, conversationID string, stepIndex int32) (*RevertPreview, error) // GetRevertPreview
	ResolveOutstandingSteps(ctx context.Context, conversationID string) error                             // ResolveOutstandingSteps

	// Agent-state streaming.
	StreamAgentStateUpdates(ctx context.Context, conversationID string) (<-chan *cortex.AgentStateComponent, error) // StreamAgentStateUpdates
	RequestAgentStatePageUpdate(ctx context.Context, conversationID string, pageIndex int32) error                  // RequestAgentStatePageUpdate

	// Summaries.
	UpdateSummaryForID(ctx context.Context, id, summary string) error           // UpdateSummaryForID
	UpdateSummaryForTrajectory(ctx context.Context, t *cortex.Trajectory) error // UpdateSummaryForTrajectory

	// Workspace resolution.
	ResolveWorkspaceURIs(ctx context.Context, conversationID string) ([]string, error)  // ResolveWorkspaceURIs
	ResolveExecutableScript(ctx context.Context, conversationID string) (string, error) // ResolveExecutableScript
	CleanupMappedWorkspaces(ctx context.Context) error                                  // CleanupMappedWorkspaces

	// BattleMode (battlemode.go).
	StartBattleMode(ctx context.Context, conversationID string, modelIDs []string) error  // StartBattleMode
	EndBattleMode(ctx context.Context, conversationID string, winnerModelID string) error // EndBattleMode

	// Code-edit acknowledgements.
	AcknowledgeCodeEdit(ctx context.Context, conversationID string, stepIndex int32) error       // AcknowledgeCodeEdit
	AcknowledgeCodeActionStep(ctx context.Context, conversationID string, stepIndex int32) error // AcknowledgeCodeActionStep
}

// ExecutionStatus is the run state of a Cascade conversation.
//
// reconstructed from: (*CascadeManager)ExecutionStatus / IsCascadeRunning
type ExecutionStatus int32

const (
	ExecutionStatusIdle ExecutionStatus = iota
	ExecutionStatusRunning
	ExecutionStatusWaitingForUser
	ExecutionStatusError
	ExecutionStatusCancelled
)

// RevertPreview is returned by GetRevertPreview.
//
// reconstructed from: (*CascadeManager)GetRevertPreview
type RevertPreview struct {
	ConversationID string
	StepIndex      int32
	AffectedFiles  []string
}

// EndBattleModeError is the recovered error type from battlemode.go.
//
// reconstructed from: EndBattleModeError.{Error,Unwrap}
type EndBattleModeError struct {
	ConversationID string
	Cause          error
}

func (e *EndBattleModeError) Error() string {
	if e == nil {
		return "cortex.EndBattleModeError: <nil>"
	}
	if e.Cause != nil {
		return "cortex.EndBattleModeError: " + e.ConversationID + ": " + e.Cause.Error()
	}
	return "cortex.EndBattleModeError: " + e.ConversationID
}

// Unwrap implements errors.Unwrap. reconstructed from: EndBattleModeError.Unwrap
func (e *EndBattleModeError) Unwrap() error {
	if e == nil {
		return nil
	}
	return e.Cause
}

// DefaultManager is a reconstructed struct skeleton for CascadeManager.
// It carries the recovered private collaborators referenced by the method set
// (battle-mode setup, env construction, descendant-conversation collection).
//
// reconstructed from: CascadeManager.{setupBattleMode,constructEnv,
//
//	collectDescendantConversations, prepareBattleModeWorkspaces, ...}
type DefaultManager struct {
	conversations map[string]ExecutionStatus
}

// NewManager mirrors the recovered CascadeManager.New constructor.
//
// reconstructed from: CascadeManager.New
func NewManager() *DefaultManager {
	return &DefaultManager{conversations: make(map[string]ExecutionStatus)}
}
