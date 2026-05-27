package cmd

import (
	"context"
	"fmt"
	"os"
	"strings"

	"google.golang.org/api/docs/v1"

	"github.com/steipete/gogcli/internal/outfmt"
	"github.com/steipete/gogcli/internal/ui"
)

// DocsInsertPageBreakCmd inserts a page break at a specific character index in
// a Google Doc (or at the end of the body/tab when --at-end is supplied, or
// --index is omitted). Surfaces the Docs API InsertPageBreakRequest directly,
// since markdown has no native page-break construct that the markdown writer
// could translate.
type DocsInsertPageBreakCmd struct {
	DocID string `arg:"" name:"docId" help:"Doc ID"`
	Index *int64 `name:"index" help:"Character index to insert at (1 = beginning). Omit or use --at-end for end-of-doc."`
	AtEnd bool   `name:"at-end" help:"Insert at end-of-doc/tab (mutually exclusive with --index)"`
	Tab   string `name:"tab" help:"Target a specific tab by title or ID (see docs list-tabs)"`
	TabID string `name:"tab-id" hidden:"" help:"(deprecated) Use --tab"`
}

func (c *DocsInsertPageBreakCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	docID := strings.TrimSpace(c.DocID)
	if docID == "" {
		return usage("empty docId")
	}
	if c.AtEnd && c.Index != nil {
		return usage("--at-end and --index are mutually exclusive")
	}
	if c.Index != nil && *c.Index < 1 {
		return usage("--index must be >= 1 (index 0 is reserved)")
	}

	tab, tabErr := resolveTabArg(ctx, c.Tab, c.TabID)
	if tabErr != nil {
		return tabErr
	}
	c.Tab = tab

	resolveEnd := c.AtEnd || c.Index == nil

	dryRunPayload := map[string]any{
		"documentId": docID,
		"tab":        c.Tab,
	}
	if resolveEnd {
		dryRunPayload["atIndex"] = docsAtIndexEnd
	} else {
		dryRunPayload["atIndex"] = *c.Index
	}
	if dryRunErr := dryRunExit(ctx, flags, "docs.insert-page-break", dryRunPayload); dryRunErr != nil {
		return dryRunErr
	}

	svc, err := requireDocsService(ctx, flags)
	if err != nil {
		return err
	}

	var insertIndex int64
	if resolveEnd {
		endIndex, tabID, endErr := docsTargetEndIndexAndTabID(ctx, svc, docID, c.Tab)
		if endErr != nil {
			return endErr
		}
		c.Tab = tabID
		insertIndex = docsAppendIndex(endIndex)
	} else {
		insertIndex = *c.Index
		if c.Tab != "" {
			tabID, tabErr := resolveDocsTabID(ctx, svc, docID, c.Tab)
			if tabErr != nil {
				return tabErr
			}
			c.Tab = tabID
		}
	}

	result, err := svc.Documents.BatchUpdate(docID, &docs.BatchUpdateDocumentRequest{
		Requests: []*docs.Request{{
			InsertPageBreak: &docs.InsertPageBreakRequest{
				Location: &docs.Location{
					Index: insertIndex,
					TabId: c.Tab,
				},
			},
		}},
	}).Context(ctx).Do()
	if err != nil {
		return fmt.Errorf("inserting page break: %w", err)
	}

	if outfmt.IsJSON(ctx) {
		payload := map[string]any{
			"documentId": result.DocumentId,
			"atIndex":    insertIndex,
		}
		if c.Tab != "" {
			payload["tabId"] = c.Tab
		}
		return outfmt.WriteJSON(ctx, os.Stdout, payload)
	}

	u.Out().Linef("documentId\t%s", result.DocumentId)
	u.Out().Linef("atIndex\t%d", insertIndex)
	if c.Tab != "" {
		u.Out().Linef("tabId\t%s", c.Tab)
	}
	return nil
}
