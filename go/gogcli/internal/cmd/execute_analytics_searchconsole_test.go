package cmd

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	analyticsadminapi "google.golang.org/api/analyticsadmin/v1beta"
	analyticsdataapi "google.golang.org/api/analyticsdata/v1beta"
	"google.golang.org/api/option"
	searchconsoleapi "google.golang.org/api/searchconsole/v1"
)

func TestExecute_AnalyticsAccounts_JSON(t *testing.T) {
	origNew := newAnalyticsAdminService
	t.Cleanup(func() { newAnalyticsAdminService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/v1beta/accountSummaries")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"accountSummaries": []map[string]any{
				{
					"account":     "accounts/123",
					"displayName": "Demo Account",
					"propertySummaries": []map[string]any{
						{"property": "properties/999", "displayName": "Main Property"},
					},
				},
			},
			"nextPageToken": "next123",
		})
	}))
	defer srv.Close()

	svc, err := analyticsadminapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newAnalyticsAdminService = func(context.Context, string) (*analyticsadminapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{"--json", "--account", "a@b.com", "analytics", "accounts", "--max", "1"}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		AccountSummaries []struct {
			Account string `json:"account"`
		} `json:"account_summaries"`
		NextPageToken string `json:"nextPageToken"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(parsed.AccountSummaries) != 1 || parsed.AccountSummaries[0].Account != "accounts/123" || parsed.NextPageToken != "next123" {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_AnalyticsAccounts_Text(t *testing.T) {
	origNew := newAnalyticsAdminService
	t.Cleanup(func() { newAnalyticsAdminService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/v1beta/accountSummaries")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"accountSummaries": []map[string]any{
				{
					"account":     "accounts/123",
					"displayName": "Demo Account",
					"propertySummaries": []map[string]any{
						{"property": "properties/999", "displayName": "Main Property"},
					},
				},
			},
			"nextPageToken": "next123",
		})
	}))
	defer srv.Close()

	svc, err := analyticsadminapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newAnalyticsAdminService = func(context.Context, string) (*analyticsadminapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{"--account", "a@b.com", "analytics", "accounts", "--max", "1"}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})
	if !strings.Contains(out, "ACCOUNT") ||
		!strings.Contains(out, "DISPLAY_NAME") ||
		!strings.Contains(out, "PROPERTIES") ||
		!strings.Contains(out, "123") ||
		!strings.Contains(out, "Demo Account") ||
		!strings.Contains(out, "1") {
		t.Fatalf("unexpected out=%q", out)
	}
}

func TestExecute_AnalyticsAccounts_AllPages_JSON(t *testing.T) {
	origNew := newAnalyticsAdminService
	t.Cleanup(func() { newAnalyticsAdminService = origNew })

	page1Calls := 0
	page2Calls := 0
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/v1beta/accountSummaries")) {
			http.NotFound(w, r)
			return
		}
		if got := r.URL.Query().Get("pageSize"); got != "1" {
			t.Fatalf("expected pageSize=1, got %q", got)
		}

		w.Header().Set("Content-Type", "application/json")
		switch r.URL.Query().Get("pageToken") {
		case "":
			page1Calls++
			_ = json.NewEncoder(w).Encode(map[string]any{
				"accountSummaries": []map[string]any{
					{"account": "accounts/111", "displayName": "One"},
				},
				"nextPageToken": "p2",
			})
		case "p2":
			page2Calls++
			_ = json.NewEncoder(w).Encode(map[string]any{
				"accountSummaries": []map[string]any{
					{"account": "accounts/222", "displayName": "Two"},
				},
				"nextPageToken": "",
			})
		default:
			t.Fatalf("unexpected pageToken=%q", r.URL.Query().Get("pageToken"))
		}
	}))
	defer srv.Close()

	svc, err := analyticsadminapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newAnalyticsAdminService = func(context.Context, string) (*analyticsadminapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--json",
				"--account", "a@b.com",
				"analytics", "accounts",
				"--all",
				"--max", "1",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		AccountSummaries []struct {
			Account string `json:"account"`
		} `json:"account_summaries"`
		NextPageToken string `json:"nextPageToken"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(parsed.AccountSummaries) != 2 ||
		parsed.AccountSummaries[0].Account != "accounts/111" ||
		parsed.AccountSummaries[1].Account != "accounts/222" ||
		parsed.NextPageToken != "" {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
	if page1Calls != 1 || page2Calls != 1 {
		t.Fatalf("unexpected page calls: page1=%d page2=%d", page1Calls, page2Calls)
	}
}

func TestExecute_AnalyticsAccounts_ServiceError(t *testing.T) {
	origNew := newAnalyticsAdminService
	t.Cleanup(func() { newAnalyticsAdminService = origNew })
	newAnalyticsAdminService = func(context.Context, string) (*analyticsadminapi.Service, error) {
		return nil, errors.New("analytics admin service down")
	}

	_ = captureStderr(t, func() {
		err := Execute([]string{"--account", "a@b.com", "analytics", "accounts"})
		if err == nil || !strings.Contains(err.Error(), "analytics admin service down") {
			t.Fatalf("unexpected err: %v", err)
		}
	})
}

func TestExecute_AnalyticsReport_Text(t *testing.T) {
	origNew := newAnalyticsDataService
	t.Cleanup(func() { newAnalyticsDataService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodPost && strings.Contains(r.URL.Path, "/v1beta/properties/123:runReport")) {
			http.NotFound(w, r)
			return
		}
		var req map[string]any
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			t.Fatalf("decode request: %v", err)
		}
		if req["limit"] != "10" {
			t.Fatalf("unexpected report limit payload: %#v", req)
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"dimensionHeaders": []map[string]any{{"name": "date"}, {"name": "country"}},
			"metricHeaders":    []map[string]any{{"name": "activeUsers"}, {"name": "sessions"}},
			"rowCount":         1,
			"rows": []map[string]any{
				{
					"dimensionValues": []map[string]any{{"value": "2026-02-01"}, {"value": "US"}},
					"metricValues":    []map[string]any{{"value": "42"}, {"value": "11"}},
				},
			},
		})
	}))
	defer srv.Close()

	svc, err := analyticsdataapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newAnalyticsDataService = func(context.Context, string) (*analyticsdataapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--account", "a@b.com",
				"analytics", "report", "123",
				"--from", "2026-02-01",
				"--to", "2026-02-01",
				"--dimensions", "date,country",
				"--metrics", "activeUsers,sessions",
				"--max", "10",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})
	if !strings.Contains(out, "DATE") ||
		!strings.Contains(out, "COUNTRY") ||
		!strings.Contains(out, "ACTIVEUSERS") ||
		!strings.Contains(out, "SESSIONS") ||
		!strings.Contains(out, "2026-02-01") ||
		!strings.Contains(out, "US") ||
		!strings.Contains(out, "42") ||
		!strings.Contains(out, "11") {
		t.Fatalf("unexpected out=%q", out)
	}
}

func TestExecute_AnalyticsReport_JSON(t *testing.T) {
	origNew := newAnalyticsDataService
	t.Cleanup(func() { newAnalyticsDataService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodPost && strings.Contains(r.URL.Path, "/v1beta/properties/123:runReport")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"dimensionHeaders": []map[string]any{{"name": "date"}},
			"metricHeaders":    []map[string]any{{"name": "activeUsers"}},
			"rowCount":         1,
			"rows": []map[string]any{
				{
					"dimensionValues": []map[string]any{{"value": "2026-02-01"}},
					"metricValues":    []map[string]any{{"value": "42"}},
				},
			},
		})
	}))
	defer srv.Close()

	svc, err := analyticsdataapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newAnalyticsDataService = func(context.Context, string) (*analyticsdataapi.Service, error) { return svc, nil }

	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			if err := Execute([]string{
				"--json",
				"--account", "a@b.com",
				"analytics", "report", "123",
				"--from", "2026-02-01",
				"--to", "2026-02-01",
				"--dimensions", "date",
				"--metrics", "activeUsers",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		Property string `json:"property"`
		From     string `json:"from"`
		To       string `json:"to"`
		RowCount int64  `json:"row_count"`
		Rows     []struct {
			DimensionValues []struct {
				Value string `json:"value"`
			} `json:"dimensionValues"`
			MetricValues []struct {
				Value string `json:"value"`
			} `json:"metricValues"`
		} `json:"rows"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if parsed.Property != "properties/123" || parsed.From != "2026-02-01" || parsed.To != "2026-02-01" || parsed.RowCount != 1 || len(parsed.Rows) != 1 {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_AnalyticsReport_FailEmpty_JSON(t *testing.T) {
	origNew := newAnalyticsDataService
	t.Cleanup(func() { newAnalyticsDataService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodPost && strings.Contains(r.URL.Path, "/v1beta/properties/123:runReport")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"dimensionHeaders": []map[string]any{{"name": "date"}},
			"metricHeaders":    []map[string]any{{"name": "activeUsers"}},
			"rowCount":         0,
			"rows":             []map[string]any{},
		})
	}))
	defer srv.Close()

	svc, err := analyticsdataapi.NewService(context.Background(),
		option.WithoutAuthentication(),
		option.WithHTTPClient(srv.Client()),
		option.WithEndpoint(srv.URL+"/"),
	)
	if err != nil {
		t.Fatalf("NewService: %v", err)
	}
	newAnalyticsDataService = func(context.Context, string) (*analyticsdataapi.Service, error) { return svc, nil }

	var execErr error
	out := captureStdout(t, func() {
		_ = captureStderr(t, func() {
			execErr = Execute([]string{
				"--json",
				"--account", "a@b.com",
				"analytics", "report", "123",
				"--from", "2026-02-01",
				"--to", "2026-02-01",
				"--dimensions", "date",
				"--metrics", "activeUsers",
				"--fail-empty",
			})
		})
	})
	if execErr == nil {
		t.Fatalf("expected error")
	}
	if got := ExitCode(execErr); got != emptyResultsExitCode {
		t.Fatalf("expected exit code %d, got %d", emptyResultsExitCode, got)
	}

	var parsed struct {
		Property string           `json:"property"`
		RowCount int64            `json:"row_count"`
		Rows     []map[string]any `json:"rows"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v\nout=%q", err, out)
	}
	if parsed.Property != "properties/123" || parsed.RowCount != 0 || len(parsed.Rows) != 0 {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_AnalyticsReport_ServiceError(t *testing.T) {
	origNew := newAnalyticsDataService
	t.Cleanup(func() { newAnalyticsDataService = origNew })
	newAnalyticsDataService = func(context.Context, string) (*analyticsdataapi.Service, error) {
		return nil, errors.New("analytics data service down")
	}

	_ = captureStderr(t, func() {
		err := Execute([]string{
			"--account", "a@b.com",
			"analytics", "report", "123",
			"--from", "2026-02-01",
			"--to", "2026-02-01",
			"--metrics", "activeUsers",
		})
		if err == nil || !strings.Contains(err.Error(), "analytics data service down") {
			t.Fatalf("unexpected err: %v", err)
		}
	})
}

func TestExecute_AnalyticsReport_ValidatesMetricsBeforeServiceCall(t *testing.T) {
	origNew := newAnalyticsDataService
	t.Cleanup(func() { newAnalyticsDataService = origNew })
	newAnalyticsDataService = func(context.Context, string) (*analyticsdataapi.Service, error) {
		t.Fatalf("expected validation to fail before creating analytics data service")
		return nil, errors.New("unexpected analytics data service call")
	}

	_ = captureStderr(t, func() {
		err := Execute([]string{
			"--account", "a@b.com",
			"analytics", "report", "123",
			"--from", "2026-02-01",
			"--to", "2026-02-01",
			"--metrics", "",
		})
		if err == nil || !strings.Contains(err.Error(), "empty --metrics") {
			t.Fatalf("unexpected err: %v", err)
		}
	})
}

func TestExecute_SearchConsoleSites_Text(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/webmasters/v3/sites")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"siteEntry": []map[string]any{
				{"siteUrl": "sc-domain:example.com", "permissionLevel": "SITE_OWNER"},
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
			if err := Execute([]string{"--account", "a@b.com", "searchconsole", "sites"}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})
	if !strings.Contains(out, "SITE") || !strings.Contains(out, "PERMISSION") || !strings.Contains(out, "sc-domain:example.com") || !strings.Contains(out, "SITE_OWNER") {
		t.Fatalf("unexpected out=%q", out)
	}
}

func TestExecute_SearchConsoleSites_JSON(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodGet && strings.Contains(r.URL.Path, "/webmasters/v3/sites")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"siteEntry": []map[string]any{
				{"siteUrl": "sc-domain:example.com", "permissionLevel": "SITE_OWNER"},
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
			if err := Execute([]string{"--json", "--account", "a@b.com", "searchconsole", "sites"}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		Sites []struct {
			SiteURL         string `json:"siteUrl"`
			PermissionLevel string `json:"permissionLevel"`
		} `json:"sites"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(parsed.Sites) != 1 || parsed.Sites[0].SiteURL != "sc-domain:example.com" || parsed.Sites[0].PermissionLevel != "SITE_OWNER" {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_SearchConsoleSites_ServiceError(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })
	newSearchConsoleService = func(context.Context, string) (*searchconsoleapi.Service, error) {
		return nil, errors.New("search console service down")
	}

	_ = captureStderr(t, func() {
		err := Execute([]string{"--account", "a@b.com", "searchconsole", "sites"})
		if err == nil || !strings.Contains(err.Error(), "search console service down") {
			t.Fatalf("unexpected err: %v", err)
		}
	})
}

func TestExecute_SearchConsoleQuery_JSON(t *testing.T) {
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
		if req["startDate"] != "2026-02-01" || req["endDate"] != "2026-02-07" || req["type"] != "WEB" {
			t.Fatalf("unexpected request payload: %#v", req)
		}
		dimensions, ok := req["dimensions"].([]any)
		if !ok || len(dimensions) != 2 || dimensions[0] != "QUERY" || dimensions[1] != "PAGE" {
			t.Fatalf("unexpected dimensions payload: %#v", req["dimensions"])
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"responseAggregationType": "AUTO",
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
				"searchconsole", "query", "sc-domain:example.com",
				"--from", "2026-02-01",
				"--to", "2026-02-07",
				"--dimensions", "query,page",
				"--type", "web",
				"--max", "10",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})

	var parsed struct {
		SiteURL string `json:"site_url"`
		Type    string `json:"type"`
		Rows    []struct {
			Keys []string `json:"keys"`
		} `json:"rows"`
	}
	if err := json.Unmarshal([]byte(out), &parsed); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if parsed.SiteURL != "sc-domain:example.com" || parsed.Type != "WEB" || len(parsed.Rows) != 1 || len(parsed.Rows[0].Keys) != 2 {
		t.Fatalf("unexpected payload: %#v", parsed)
	}
}

func TestExecute_SearchConsoleQuery_Text(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !(r.Method == http.MethodPost && strings.Contains(r.URL.Path, "/searchAnalytics/query")) {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"responseAggregationType": "AUTO",
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
				"--account", "a@b.com",
				"searchconsole", "query", "sc-domain:example.com",
				"--from", "2026-02-01",
				"--to", "2026-02-07",
				"--dimensions", "query,page",
				"--type", "web",
				"--max", "10",
			}); err != nil {
				t.Fatalf("Execute: %v", err)
			}
		})
	})
	if !strings.Contains(out, "QUERY") ||
		!strings.Contains(out, "PAGE") ||
		!strings.Contains(out, "CLICKS") ||
		!strings.Contains(out, "IMPRESSIONS") ||
		!strings.Contains(out, "CTR") ||
		!strings.Contains(out, "POSITION") ||
		!strings.Contains(out, "gog cli") ||
		!strings.Contains(out, "https://example.com/docs") ||
		!strings.Contains(out, "12") ||
		!strings.Contains(out, "300") {
		t.Fatalf("unexpected out=%q", out)
	}
}

func TestExecute_SearchConsoleQuery_ServiceError(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })
	newSearchConsoleService = func(context.Context, string) (*searchconsoleapi.Service, error) {
		return nil, errors.New("search console service down")
	}

	_ = captureStderr(t, func() {
		err := Execute([]string{
			"--account", "a@b.com",
			"searchconsole", "query", "sc-domain:example.com",
			"--from", "2026-02-01",
			"--to", "2026-02-07",
		})
		if err == nil || !strings.Contains(err.Error(), "search console service down") {
			t.Fatalf("unexpected err: %v", err)
		}
	})
}

func TestExecute_SearchConsoleQuery_ValidatesDateBeforeServiceCall(t *testing.T) {
	origNew := newSearchConsoleService
	t.Cleanup(func() { newSearchConsoleService = origNew })
	newSearchConsoleService = func(context.Context, string) (*searchconsoleapi.Service, error) {
		t.Fatalf("expected validation to fail before creating search console service")
		return nil, errors.New("unexpected search console service call")
	}

	_ = captureStderr(t, func() {
		err := Execute([]string{
			"--account", "a@b.com",
			"searchconsole", "query", "sc-domain:example.com",
			"--from", "2026/02/01",
			"--to", "2026-02-07",
		})
		if err == nil || !strings.Contains(err.Error(), "invalid --from") {
			t.Fatalf("unexpected err: %v", err)
		}
	})
}
