package googleapi

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestNewYouTubeAPIKeyHTTPClientAddsKeyWithRetryTransport(t *testing.T) {
	client, err := newYouTubeAPIKeyHTTPClient(context.Background(), "test-key")
	if err != nil {
		t.Fatalf("newYouTubeAPIKeyHTTPClient: %v", err)
	}

	if _, ok := client.Transport.(*RetryTransport); !ok {
		t.Fatalf("transport = %T, want *RetryTransport", client.Transport)
	}

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if got := r.URL.Query().Get("key"); got != "test-key" {
			t.Fatalf("key = %q", got)
		}

		if got := r.URL.Query().Get("part"); got != "snippet" {
			t.Fatalf("part = %q", got)
		}

		w.WriteHeader(http.StatusNoContent)
	}))
	defer srv.Close()

	req, err := http.NewRequestWithContext(context.Background(), http.MethodGet, srv.URL+"/youtube/v3/videos?part=snippet", nil)
	if err != nil {
		t.Fatalf("NewRequestWithContext: %v", err)
	}

	resp, err := client.Do(req)
	if err != nil {
		t.Fatalf("Do: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusNoContent {
		t.Fatalf("status = %d", resp.StatusCode)
	}
}
