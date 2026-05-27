package cmd

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"os"
	"strings"

	"google.golang.org/api/drive/v3"
	gapi "google.golang.org/api/googleapi"

	"github.com/steipete/gogcli/internal/outfmt"
	"github.com/steipete/gogcli/internal/ui"
)

const driveChangesFields = "nextPageToken,newStartPageToken,changes(kind,type,removed,time,fileId,driveId,file(id,name,mimeType,modifiedTime,trashed,webViewLink))"

type DriveChangesCmd struct {
	StartToken DriveChangesStartTokenCmd `cmd:"" name:"start-token" aliases:"token" help:"Get a Drive changes start page token"`
	List       DriveChangesListCmd       `cmd:"" name:"list" aliases:"ls" help:"List Drive changes since a page token"`
	Watch      DriveChangesWatchCmd      `cmd:"" name:"watch" help:"Watch Drive changes with a webhook channel"`
	Stop       DriveChangesStopCmd       `cmd:"" name:"stop" help:"Stop a Drive changes webhook channel"`
}

type DriveChangesStartTokenCmd struct {
	DriveID string `name:"drive" aliases:"drive-id" help:"Shared drive ID for a shared-drive change log"`
}

func (c *DriveChangesStartTokenCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	_, svc, err := requireDriveService(ctx, flags)
	if err != nil {
		return err
	}

	call := svc.Changes.GetStartPageToken().SupportsAllDrives(true).Context(ctx)
	if driveID := strings.TrimSpace(c.DriveID); driveID != "" {
		call = call.DriveId(driveID)
	}
	resp, err := call.Do()
	if err != nil {
		return err
	}

	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"startPageToken": resp.StartPageToken})
	}
	u.Out().Linef("startPageToken\t%s", resp.StartPageToken)
	return nil
}

type DriveChangesListCmd struct {
	Token          string `name:"token" required:"" help:"Start page token or next page token"`
	Max            int64  `name:"max" aliases:"limit" help:"Max results" default:"100"`
	Page           string `name:"page" aliases:"cursor" help:"Alias for --token when continuing a page"`
	All            bool   `name:"all" aliases:"all-pages,allpages" help:"Fetch all pages"`
	IncludeRemoved bool   `name:"include-removed" help:"Include removed changes" default:"true" negatable:"_"`
	DriveID        string `name:"drive" aliases:"drive-id" help:"Shared drive ID for a shared-drive change log"`
	FailEmpty      bool   `name:"fail-empty" aliases:"non-empty,require-results" help:"Exit with code 3 if no changes"`
}

func (c *DriveChangesListCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	_, svc, err := requireDriveService(ctx, flags)
	if err != nil {
		return err
	}
	token := strings.TrimSpace(c.Page)
	if token == "" {
		token = strings.TrimSpace(c.Token)
	}
	if token == "" {
		return usage("missing --token")
	}

	fetch := func(pageToken string) ([]*drive.Change, string, error) {
		call := svc.Changes.List(pageToken).
			PageSize(c.Max).
			IncludeItemsFromAllDrives(true).
			SupportsAllDrives(true).
			IncludeRemoved(c.IncludeRemoved).
			Fields(gapi.Field(driveChangesFields)).
			Context(ctx)
		if driveID := strings.TrimSpace(c.DriveID); driveID != "" {
			call = call.DriveId(driveID)
		}
		resp, callErr := call.Do()
		if callErr != nil {
			return nil, "", callErr
		}
		next := resp.NextPageToken
		if next == "" {
			next = resp.NewStartPageToken
		}
		return resp.Changes, next, nil
	}

	changes, nextPageToken, err := loadPagedItems(token, c.All, fetch)
	if err != nil {
		return err
	}

	if outfmt.IsJSON(ctx) {
		return writePagedJSONResult(ctx, map[string]any{
			"changes":       changes,
			"nextPageToken": nextPageToken,
		}, len(changes), c.FailEmpty)
	}
	if len(changes) == 0 {
		u.Err().Println("No changes")
		return failEmptyExit(c.FailEmpty)
	}

	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, "TIME\tTYPE\tFILE_ID\tNAME\tREMOVED")
	for _, change := range changes {
		name := ""
		if change.File != nil {
			name = change.File.Name
		}
		fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%t\n", change.Time, change.Type, change.FileId, sanitizeTab(name), change.Removed)
	}
	printNextPageHint(u, nextPageToken)
	return nil
}

type DriveChangesWatchCmd struct {
	Token        string `name:"token" required:"" help:"Start page token or next page token to watch from"`
	WebhookURL   string `name:"webhook-url" required:"" help:"HTTPS webhook URL for Drive change notifications"`
	ChannelID    string `name:"channel-id" help:"Webhook channel ID (default: generated)"`
	ChannelToken string `name:"channel-token" help:"Opaque token echoed by Google in webhook notifications"`
	ExpirationMS int64  `name:"expiration-ms" help:"Unix epoch milliseconds when the channel should expire"`
	DriveID      string `name:"drive" aliases:"drive-id" help:"Shared drive ID for a shared-drive change log"`
}

func (c *DriveChangesWatchCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	token := strings.TrimSpace(c.Token)
	webhookURL := strings.TrimSpace(c.WebhookURL)
	if token == "" {
		return usage("missing --token")
	}
	if webhookURL == "" {
		return usage("missing --webhook-url")
	}
	channelID := strings.TrimSpace(c.ChannelID)
	if channelID == "" {
		var err error
		channelID, err = randomChannelID()
		if err != nil {
			return err
		}
	}
	channel := &drive.Channel{
		Id:      channelID,
		Type:    "web_hook",
		Address: webhookURL,
		Token:   strings.TrimSpace(c.ChannelToken),
	}
	if c.ExpirationMS > 0 {
		channel.Expiration = c.ExpirationMS
	}
	driveID := strings.TrimSpace(c.DriveID)
	channelTokenState := ""
	if channel.Token != "" {
		channelTokenState = "provided"
	}

	if err := dryRunExit(ctx, flags, "drive.changes.watch", map[string]any{
		"token":         token,
		"webhook_url":   webhookURL,
		"channel_id":    channelID,
		"channel_token": channelTokenState,
		"expiration_ms": c.ExpirationMS,
		"drive_id":      driveID,
	}); err != nil {
		return err
	}

	_, svc, err := requireDriveService(ctx, flags)
	if err != nil {
		return err
	}

	call := svc.Changes.Watch(token, channel).
		SupportsAllDrives(true).
		Context(ctx)
	if driveID != "" {
		call = call.DriveId(driveID)
	}
	resp, err := call.Do()
	if err != nil {
		return err
	}
	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"channel": resp})
	}
	u.Out().Linef("id\t%s", resp.Id)
	u.Out().Linef("resourceId\t%s", resp.ResourceId)
	u.Out().Linef("resourceUri\t%s", resp.ResourceUri)
	u.Out().Linef("expiration\t%d", resp.Expiration)
	return nil
}

type DriveChangesStopCmd struct {
	ChannelID  string `arg:"" name:"channelId" help:"Webhook channel ID"`
	ResourceID string `arg:"" name:"resourceId" help:"Webhook resource ID returned by watch"`
}

func (c *DriveChangesStopCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	channelID := strings.TrimSpace(c.ChannelID)
	resourceID := strings.TrimSpace(c.ResourceID)
	if channelID == "" || resourceID == "" {
		return usage("required: channelId resourceId")
	}

	if err := dryRunExit(ctx, flags, "drive.changes.stop", map[string]any{
		"channel_id":  channelID,
		"resource_id": resourceID,
	}); err != nil {
		return err
	}

	_, svc, err := requireDriveService(ctx, flags)
	if err != nil {
		return err
	}
	if err := svc.Channels.Stop(&drive.Channel{Id: channelID, ResourceId: resourceID}).Context(ctx).Do(); err != nil {
		return err
	}
	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{"stopped": true, "channelId": channelID, "resourceId": resourceID})
	}
	u.Out().Linef("stopped\ttrue")
	return nil
}

func randomChannelID() (string, error) {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "", err
	}
	return "gog-" + hex.EncodeToString(b[:]), nil
}
