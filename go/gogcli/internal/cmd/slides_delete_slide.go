package cmd

import (
	"context"
	"fmt"
	"strings"

	"google.golang.org/api/slides/v1"

	"github.com/steipete/gogcli/internal/ui"
)

type SlidesDeleteSlideCmd struct {
	PresentationID string `arg:"" name:"presentationId" help:"Presentation ID"`
	SlideID        string `arg:"" name:"slideId" help:"Slide object ID to delete (use 'slides list-slides' to find IDs)"`
}

func (c *SlidesDeleteSlideCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)

	presentationID := strings.TrimSpace(c.PresentationID)
	if presentationID == "" {
		return usage("empty presentationId")
	}
	slideID := strings.TrimSpace(c.SlideID)
	if slideID == "" {
		return usage("empty slideId")
	}

	if err := dryRunExit(ctx, flags, "slides.delete-slide", map[string]any{
		"presentation_id": presentationID,
		"slide_id":        slideID,
	}); err != nil {
		return err
	}

	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	slidesSvc, err := newSlidesService(ctx, account)
	if err != nil {
		return err
	}

	_, err = slidesSvc.Presentations.BatchUpdate(presentationID, &slides.BatchUpdatePresentationRequest{
		Requests: []*slides.Request{
			{
				DeleteObject: &slides.DeleteObjectRequest{
					ObjectId: slideID,
				},
			},
		},
	}).Context(ctx).Do()
	if err != nil {
		return fmt.Errorf("delete slide: %w", err)
	}

	u.Out().Linef("Deleted slide %s", slideID)
	return nil
}
