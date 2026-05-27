package cmd

import (
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"testing"

	"google.golang.org/api/drive/v3"
	"google.golang.org/api/driveactivity/v2"
)

func TestDriveChangesStartToken(t *testing.T) {
	svc, closeSrv := newDriveTestService(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/changes/startPageToken" {
			t.Fatalf("path = %q", r.URL.Path)
		}
		requireQuery(t, r, "supportsAllDrives", "true")
		_ = json.NewEncoder(w).Encode(map[string]any{"startPageToken": "123"})
	}))
	defer closeSrv()
	stubGoogleTestService(t, &newDriveService, svc)

	if err := (&DriveChangesStartTokenCmd{}).Run(newCmdOutputContext(t, io.Discard, io.Discard), &RootFlags{Account: "a@example.com"}); err != nil {
		t.Fatalf("Run: %v", err)
	}
}

func TestDriveChangesList(t *testing.T) {
	svc, closeSrv := newDriveTestService(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/changes" {
			t.Fatalf("path = %q", r.URL.Path)
		}
		requireQuery(t, r, "pageToken", "123")
		requireQuery(t, r, "includeItemsFromAllDrives", "true")
		requireQuery(t, r, "supportsAllDrives", "true")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"newStartPageToken": "456",
			"changes": []map[string]any{{
				"type":   "file",
				"fileId": "file1",
				"file":   map[string]any{"id": "file1", "name": "Doc"},
			}},
		})
	}))
	defer closeSrv()
	stubGoogleTestService(t, &newDriveService, svc)

	if err := (&DriveChangesListCmd{Token: "123", Max: 10, IncludeRemoved: true}).Run(newCmdOutputContext(t, io.Discard, io.Discard), &RootFlags{Account: "a@example.com"}); err != nil {
		t.Fatalf("Run: %v", err)
	}
}

func TestDriveActivityFilter(t *testing.T) {
	filter, err := driveActivityFilter("edit,share", "2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z", `detail.action_detail_case:-MOVE`)
	if err != nil {
		t.Fatalf("driveActivityFilter: %v", err)
	}
	for _, want := range []string{
		`time >= "2026-01-01T00:00:00Z"`,
		`time <= "2026-01-02T00:00:00Z"`,
		"detail.action_detail_case:(EDIT PERMISSION_CHANGE)",
		"detail.action_detail_case:-MOVE",
	} {
		if !strings.Contains(filter, want) {
			t.Fatalf("filter %q missing %q", filter, want)
		}
	}
}

func TestDriveActivityQuery(t *testing.T) {
	svc, closeSrv := newGoogleTestService(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/v2/activity:query" {
			t.Fatalf("path = %q", r.URL.Path)
		}
		var req driveactivity.QueryDriveActivityRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		if req.ItemName != "items/file1" || !strings.Contains(req.Filter, "EDIT") {
			t.Fatalf("unexpected request: %#v", req)
		}
		_ = json.NewEncoder(w).Encode(map[string]any{
			"activities": []map[string]any{{
				"timestamp":           "2026-01-01T00:00:00Z",
				"primaryActionDetail": map[string]any{"edit": map[string]any{}},
				"targets":             []map[string]any{{"driveItem": map[string]any{"name": "items/file1", "title": "Doc"}}},
			}},
		})
	}), driveactivity.NewService)
	defer closeSrv()
	stubGoogleTestService(t, &newDriveActivityService, svc)

	if err := (&DriveActivityQueryCmd{File: "file1", Actions: "edit", Max: 10}).Run(newCmdOutputContext(t, io.Discard, io.Discard), &RootFlags{Account: "a@example.com"}); err != nil {
		t.Fatalf("Run: %v", err)
	}
}

var _ = drive.Change{}
