package cmd

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"google.golang.org/api/option"
	searchconsoleapi "google.golang.org/api/searchconsole/v1"
)

func TestSearchConsoleQueryCmd_BuildRequest(t *testing.T) {
	cmd := &SearchConsoleQueryCmd{
		From:        "2026-02-01",
		To:          "2026-02-28",
		Dimensions:  "query,page",
		Type:        "web",
		Aggregation: "by_page",
		DataState:   "final",
		Max:         250,
		Offset:      10,
		Filter:      []string{"query:contains:buy shoes", "country:equals:usa"},
	}

	req, err := cmd.buildRequest()
	if err != nil {
		t.Fatalf("buildRequest: %v", err)
	}

	if req.StartDate != "2026-02-01" || req.EndDate != "2026-02-28" {
		t.Fatalf("unexpected date range: %s - %s", req.StartDate, req.EndDate)
	}
	if req.RowLimit != 250 || req.StartRow != 10 {
		t.Fatalf("unexpected pagination: limit=%d startRow=%d", req.RowLimit, req.StartRow)
	}
	if len(req.Dimensions) != 2 || req.Dimensions[0] != "QUERY" || req.Dimensions[1] != "PAGE" {
		t.Fatalf("unexpected dimensions: %#v", req.Dimensions)
	}
	if req.Type != "WEB" || req.AggregationType != "BY_PAGE" || req.DataState != "FINAL" {
		t.Fatalf("unexpected query options: %#v", req)
	}
	if len(req.DimensionFilterGroups) != 1 || req.DimensionFilterGroups[0].GroupType != "AND" {
		t.Fatalf("unexpected filter groups: %#v", req.DimensionFilterGroups)
	}
	if len(req.DimensionFilterGroups[0].Filters) != 2 {
		t.Fatalf("unexpected filter count: %#v", req.DimensionFilterGroups[0].Filters)
	}
	if req.DimensionFilterGroups[0].Filters[0].Dimension != "QUERY" || req.DimensionFilterGroups[0].Filters[0].Operator != "CONTAINS" {
		t.Fatalf("unexpected first filter: %#v", req.DimensionFilterGroups[0].Filters[0])
	}
}

func TestSearchConsoleQueryCmd_BuildRequestFromJSON(t *testing.T) {
	withStdin(t, `{
		"startDate":"2026-02-01",
		"endDate":"2026-02-10",
		"rowLimit":50,
		"searchType":"IMAGE",
		"dimensions":["QUERY","search_appearance"],
		"dimensionFilterGroups":[{"groupType":"AND","filters":[{"dimension":"PAGE","operator":"NOT_CONTAINS","expression":"draft"}]}]
	}`, func() {
		cmd := &SearchConsoleQueryCmd{Request: "-"}
		req, err := cmd.buildRequest()
		if err != nil {
			t.Fatalf("buildRequest: %v", err)
		}
		if req.RowLimit != 50 {
			t.Fatalf("unexpected rowLimit: %d", req.RowLimit)
		}
		if req.Type != "IMAGE" || req.SearchType != "IMAGE" {
			t.Fatalf("unexpected type fields: type=%q searchType=%q", req.Type, req.SearchType)
		}
		if len(req.Dimensions) != 2 || req.Dimensions[0] != "QUERY" || req.Dimensions[1] != "SEARCH_APPEARANCE" {
			t.Fatalf("unexpected dimensions: %#v", req.Dimensions)
		}
		if len(req.DimensionFilterGroups) != 1 || req.DimensionFilterGroups[0].GroupType != "AND" {
			t.Fatalf("unexpected filter groups: %#v", req.DimensionFilterGroups)
		}
		filter := req.DimensionFilterGroups[0].Filters[0]
		if filter.Dimension != "PAGE" || filter.Operator != "NOT_CONTAINS" || filter.Expression != "draft" {
			t.Fatalf("unexpected filter: %#v", filter)
		}
	})
}

func TestExecute_SearchConsoleSitesGet_JSON(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/webmasters/v3/sites/")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"siteUrl":         "sc-domain:example.com",
			"permissionLevel": "SITE_OWNER",
		})
	}))
	defer srv.Close()

	svc, err := searchconsoleapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newSearchConsoleService = func(context.Context, string) (*searchconsoleapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--json",
				"--account", "a@b.com",
				"searchconsole", "sites", "get", "sc-domain:example.com",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		Site struct {
			SiteURL         string `json:"siteUrl"`
			PermissionLevel string `json:"permissionLevel"`
		} `json:"site"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if parsed.Site.SiteURL != "sc-domain:example.com" || parsed.Site.PermissionLevel != "SITE_OWNER" {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_SearchConsoleSearchAnalyticsQuery_JSON(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodPost && strings.Contains(r.URL.Path, "/searchAnalytics/query")) {
			http.NotFound(w, r)
			return
		}
		var req map[string]any
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("decode request: %v", err)
		}
		if req["aggregationType"] != "BY_PAGE" || req["dataState"] != "FINAL" || req["type"] != "WEB" {
			t.Fatalf("unexpected request payload: %#v", req)
		}
		filterGroups, ok := req["dimensionFilterGroups"].([]any)
		if !ok || len(filterGroups) != 1 {
			t.Fatalf("unexpected filter groups: %#v", req["dimensionFilterGroups"])
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"responseAggregationType": "BY_PAGE",
			"rows": []map[string]any{
				{
					"keys":        []string{"gog cli", "https://example.com/docs"},
					"clicks":      12,
					"impressions": 300,
					"ctr":         0.04,
					"position":    7.3,
				},
			},
		})
	}))
	defer srv.Close()

	svc, err := searchconsoleapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newSearchConsoleService = func(context.Context, string) (*searchconsoleapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--json",
				"--account", "a@b.com",
				"searchconsole", "searchanalytics", "query", "sc-domain:example.com",
				"--from", "2026-02-01",
				"--to", "2026-02-07",
				"--dimensions", "query,page",
				"--type", "web",
				"--aggregation", "by_page",
				"--data-state", "final",
				"--filter", "query:contains:gog",
				"--max", "10",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		Type                    string `json:"type"`
		ResponseAggregationType string `json:"response_aggregation_type"`
		Rows                    []struct {
			Keys []string `json:"keys"`
		} `json:"rows"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if parsed.Type != "WEB" || parsed.ResponseAggregationType != "BY_PAGE" || len(parsed.Rows) != 1 {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_SearchConsoleSitemapsList_JSON(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/sitemaps")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"sitemap": []map[string]any{
				{
					"path":           "https://example.com/sitemap.xml",
					"type":           "SITEMAP",
					"isPending":      false,
					"warnings":       "1",
					"errors":         "0",
					"lastSubmitted":  "2026-02-01",
					"lastDownloaded": "2026-02-02",
				},
			},
		})
	}))
	defer srv.Close()

	svc, err := searchconsoleapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newSearchConsoleService = func(context.Context, string) (*searchconsoleapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--json",
				"--account", "a@b.com",
				"searchconsole", "sitemaps", "sc-domain:example.com",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		Sitemaps []struct {
			Path string `json:"path"`
			Type string `json:"type"`
		} `json:"sitemaps"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(parsed.Sitemaps) != 1 || parsed.Sitemaps[0].Path != "https://example.com/sitemap.xml" || parsed.Sitemaps[0].Type != "SITEMAP" {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_SearchConsoleSitemapsSubmit_DryRun_JSON(t *testing.T) {
	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--json",
				"--dry-run",
				"searchconsole", "sitemaps", "submit",
				"sc-domain:example.com",
				"https://example.com/sitemap.xml",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		DryRun bool   `json:"dry_run"`
		Op     string `json:"op"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if !parsed.DryRun || parsed.Op != "searchconsole.sitemaps.submit" {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}
