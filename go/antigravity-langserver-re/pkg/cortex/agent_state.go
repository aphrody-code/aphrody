// SPDX-License-Identifier: Apache-2.0

package cortex

// Trajectory is the agent's conversation/run record (the "trajectory").
//
// reconstructed from: cortex/cortex trajectory.go + cortex/trajectory/trajectory
//
//	and the LanguageServerService Get*Trajectory* / *CascadeTrajectory* methods.
type Trajectory struct {
	ID             string
	ConversationID string
	Steps          []*TrajectoryStep
	Summary        string
	ModelID        string
}

// TrajectoryStep is one step in a trajectory (a tool call, a model turn, a user
// input, etc.).
//
// reconstructed from: CascadeManager.{GetTrajectory,RevertToStep,ResolveOutstandingSteps}
type TrajectoryStep struct {
	Index    int32
	Kind     StepKind
	ToolName string // set when Kind == StepKindToolCall
	Content  string
}

// StepKind classifies a trajectory step.
//
// reconstructed from: cortex agent_state.go / trajectory.go step handling
type StepKind int32

const (
	StepKindUnknown StepKind = iota
	StepKindUserInput
	StepKindModelTurn
	StepKindToolCall
	StepKindToolResult
)

// AgentStateComponent mirrors cortex/agent_state_component — a reactive piece of
// agent UI state streamed to the panel.
//
// reconstructed from: cortex/agent_state_component/agent_state_component +
//
//	LanguageServerService.{StreamAgentStateUpdates,RequestAgentStatePageUpdate}
type AgentStateComponent struct {
	ConversationID string
	PageIndex      int32
	Payload        []byte
}

// AnnotationsManager mirrors cortex annotations_manager.go.
//
// reconstructed from: cortex/cortex annotations_manager.go
type AnnotationsManager struct {
	conversationID string
}

// SummariesStore mirrors cortex summaries_store.go (state-sync summaries).
//
// reconstructed from: cortex/cortex summaries_store.go + cortex.stateSyncSummariesStore
type SummariesStore struct{}
