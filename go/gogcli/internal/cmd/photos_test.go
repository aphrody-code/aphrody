package cmd

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/steipete/gogcli/internal/googleapi"
)

func TestPhotosSearchBuildsReadOnlyRequest(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost || r.URL.Path != "/mediaItems:search" {
			http.NotFound(w, r)
			return
		}
		var body map[string]any
		if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
			t.Fatalf("decode body: %v", err)
		}
		if body["pageSize"].(float64) != 10 {
			t.Fatalf("unexpected body: %#v", body)
		}
		filters := body["filters"].(map[string]any)
		mt := filters["mediaTypeFilter"].(map[string]any)
		if got := mt["mediaTypes"].([]any)[0]; got != "PHOTO" {
			t.Fatalf("media type = %v", got)
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"mediaItems": []map[string]any{{
				"id":         "m1",
				"filename":   "photo.jpg",
				"mimeType":   "image/jpeg",
				"productUrl": "https://photos.example/m1",
				"mediaMetadata": map[string]any{
					"creationTime": "2026-01-01T00:00:00Z",
				},
			}},
		})
	}))
	defer srv.Close()

	orig := newPhotosClient
	t.Cleanup(func() { newPhotosClient = orig })
	newPhotosClient = func(context.Context, string) (*googleapi.PhotosClient, error) {
		return googleapi.NewPhotosClient(srv.Client(), googleapi.WithPhotosBaseURL(srv.URL)), nil
	}

	out := captureStdout(t, func() {
		cmd := &PhotosSearchCmd{MediaType: "PHOTO", Max: 10, From: "2026-01-01", To: "2026-01-02"}
		if err := cmd.Run(newCalendarJSONContext(t), &RootFlags{Account: "a@example.com"}); err != nil {
			t.Fatalf("Run: %v", err)
		}
	})
	var parsed struct {
		MediaItemCount int `json:"mediaItemCount"`
		MediaItems     []struct {
			ID string `json:"id"`
		} `json:"mediaItems"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("json parse: %v\n%s", err, out)
	}
	if parsed.MediaItemCount != 1 || len(parsed.MediaItems) != 1 || parsed.MediaItems[0].ID != "m1" {
		t.Fatalf("unexpected output: %#v", parsed)
	}
}
