package cmd

import (
	"context"
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"strings"
	"testing"

	"google.golang.org/api/drive/v3"
	"google.golang.org/api/drivelabels/v2"
)

func TestDriveLabelsList_JSON(t *testing.T) {
	svc, closeSvc := newGoogleTestService(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet || r.URL.Path != "/v2/labels" {
			http.NotFound(w, r)
			return
		}
		requireQuery(t, r, "publishedOnly", "true")
		requireQuery(t, r, "view", "LABEL_VIEW_BASIC")
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"labels": []map[string]any{{
				"name":       "labels/abc",
				"id":         "abc",
				"revisionId": "1",
				"labelType":  "SHARED",
				"properties": map[string]any{"title": "Project"},
				"lifecycle":  map[string]any{"state": "PUBLISHED"},
			}},
		})
	}), drivelabels.NewService)
	defer closeSvc()
	orig := newDriveLabelsService
	t.Cleanup(func() { newDriveLabelsService = orig })
	newDriveLabelsService = func(context.Context, string) (*drivelabels.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		if err := (&DriveLabelsListCmd{PublishedOnly: true, View: "LABEL_VIEW_BASIC"}).Run(newCalendarJSONContext(t), &RootFlags{Account: "a@example.com"}); err != nil {
			t.Fatalf("Run: %v", err)
		}
	})
	var parsed struct {
		LabelCount int `json:"labelCount"`
		Labels     []struct {
			Name string `json:"name"`
		} `json:"labels"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("json parse: %v\n%s", err, out)
	}
	if parsed.LabelCount != 1 || len(parsed.Labels) != 1 || parsed.Labels[0].Name != "labels/abc" {
		t.Fatalf("unexpected output: %#v", parsed)
	}
}

func TestNormalizeDriveLabelName(t *testing.T) {
	if got := normalizeDriveLabelName("abc"); got != "labels/abc" {
		t.Fatalf("unexpected: %q", got)
	}
	if got := normalizeDriveLabelName("labels/abc"); got != "labels/abc" {
		t.Fatalf("unexpected: %q", got)
	}
}

func TestDriveLabelsFileList_JSON(t *testing.T) {
	svc, closeSvc := newDriveTestService(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet || r.URL.Path != "/files/file1/listLabels" {
			http.NotFound(w, r)
			return
		}
		requireQuery(t, r, "maxResults", "10")
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"labels": []map[string]any{{"id": "label1", "revisionId": "2"}},
		})
	}))
	defer closeSvc()
	stubGoogleTestService(t, &newDriveService, svc)

	out := captureStdout(t, func() {
		if err := (&DriveLabelsFileListCmd{FileID: "file1", Max: 10}).Run(newCalendarJSONContext(t), &RootFlags{Account: "a@example.com"}); err != nil {
			t.Fatalf("Run: %v", err)
		}
	})
	if !strings.Contains(out, `"labelCount": 1`) || !strings.Contains(out, "label1") {
		t.Fatalf("unexpected output: %s", out)
	}
}

func TestDriveLabelsFileApply_BuildsModifyLabelsRequest(t *testing.T) {
	svc, closeSvc := newDriveTestService(t, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/files/file1/modifyLabels" {
			http.NotFound(w, r)
			return
		}
		var req drive.ModifyLabelsRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil && err != io.EOF {
			t.Fatalf("decode body: %v", err)
		}
		if len(req.LabelModifications) != 1 || req.LabelModifications[0].LabelId != "label1" {
			t.Fatalf("unexpected request: %#v", req)
		}
		fields := req.LabelModifications[0].FieldModifications
		if len(fields) != 2 || fields[0].FieldId != "title" || fields[0].SetTextValues[0] != "Project" || !fields[1].UnsetValues {
			t.Fatalf("unexpected fields: %#v", fields)
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"modifiedLabels": []map[string]any{{"id": "label1"}},
		})
	}))
	defer closeSvc()
	stubGoogleTestService(t, &newDriveService, svc)

	if err := (&DriveLabelsFileApplyCmd{
		FileID:  "file1",
		LabelID: "labels/label1",
		Text:    []string{"title=Project"},
		Unset:   []string{"old"},
	}).Run(newCmdOutputContext(t, io.Discard, io.Discard), &RootFlags{Account: "a@example.com", Force: true}); err != nil {
		t.Fatalf("Run: %v", err)
	}
}

func TestWrapDriveLabelsErrorValidCustomer(t *testing.T) {
	err := wrapDriveLabelsError(errors.New("Cannot perform this action without a valid customer"))
	if err == nil || !strings.Contains(err.Error(), "requires a Google Workspace customer") {
		t.Fatalf("unexpected error: %v", err)
	}
}
