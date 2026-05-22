// SPDX-License-Identifier: Apache-2.0

package cortex

// ToolConverter is the common shape of the agent's tool adapters. Each concrete
// converter in cortex/tools/tools adapts one agent tool (the model's function
// calls) to/from the IDE chat representation.
//
// reconstructed from: cortex/tools/tools *ToolConverter type set
type ToolConverter interface {
	// ToolName returns the canonical tool identifier (e.g. "run_command").
	ToolName() string
}

// Tool identifiers, one per recovered *ToolConverter symbol. These are the
// tools the Cascade agent can invoke.
//
// reconstructed from: cortex/tools/tools (AskPermissionToolConverter,
//
//	AskQuestionToolConverter, CallMcpToolConverter, CommandStatusToolConverter,
//	FindToolConverter, FinishToolConverter, GenerateImageToolConverter,
//	GrepSearchToolConverter, ListDirToolConverter, ListPermissionsToolConverter,
//	ListResourcesToolConverter, ManageInboxToolConverter, ManageTaskToolConverter,
//	McpToolConverter, ReadResourceToolConverter, RunCommandToolConverter,
//	ScheduleToolConverter, SearchWebToolConverter, SedFileToolConverter,
//	SendCommandInputToolConverter, SendMessageToolConverter, ViewFileToolConverter,
//	WaitToolConverter, WaitFiveSecondsToolConverter, WorkspaceAPIToolConverter,
//	ReplaceFileContentToolConverter)
const (
	ToolAskPermission    = "ask_permission"
	ToolAskQuestion      = "ask_question"
	ToolCallMcp          = "call_mcp"
	ToolCommandStatus    = "command_status"
	ToolFind             = "find"
	ToolFinish           = "finish"
	ToolGenerateImage    = "generate_image"
	ToolGrepSearch       = "grep_search"
	ToolListDir          = "list_dir"
	ToolListPermissions  = "list_permissions"
	ToolListResources    = "list_resources"
	ToolManageInbox      = "manage_inbox"
	ToolManageTask       = "manage_task"
	ToolMcp              = "mcp"
	ToolReadResource     = "read_resource"
	ToolReplaceFile      = "replace_file_content"
	ToolRunCommand       = "run_command"
	ToolSchedule         = "schedule"
	ToolSearchWeb        = "search_web"
	ToolSedFile          = "sed_file"
	ToolSendCommandInput = "send_command_input"
	ToolSendMessage      = "send_message"
	ToolViewFile         = "view_file"
	ToolWait             = "wait"
	ToolWaitFiveSeconds  = "wait_five_seconds"
	ToolWorkspaceAPI     = "workspace_api"
)

// AllTools lists every reconstructed agent tool identifier.
var AllTools = []string{
	ToolAskPermission, ToolAskQuestion, ToolCallMcp, ToolCommandStatus,
	ToolFind, ToolFinish, ToolGenerateImage, ToolGrepSearch, ToolListDir,
	ToolListPermissions, ToolListResources, ToolManageInbox, ToolManageTask,
	ToolMcp, ToolReadResource, ToolReplaceFile, ToolRunCommand, ToolSchedule,
	ToolSearchWeb, ToolSedFile, ToolSendCommandInput, ToolSendMessage,
	ToolViewFile, ToolWait, ToolWaitFiveSeconds, ToolWorkspaceAPI,
}
