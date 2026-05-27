package cmd

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strconv"
	"strings"
	"time"

	gapi "google.golang.org/api/googleapi"
	searchconsoleapi "google.golang.org/api/searchconsole/v1"

	"github.com/steipete/gogcli/internal/config"
	"github.com/steipete/gogcli/internal/googleapi"
	"github.com/steipete/gogcli/internal/outfmt"
	"github.com/steipete/gogcli/internal/ui"
)

var newSearchConsoleService = googleapi.NewSearchConsole

const (
	defaultSearchConsoleRowLimit = int64(1000)
	maxSearchConsoleRowLimit     = int64(25000)

	searchConsoleGroupAnd          = "AND"
	searchConsoleTypeWeb           = "WEB"
	searchConsoleAggregationByPage = "BY_PAGE"
	searchConsoleDimensionQuery    = "QUERY"
	searchConsoleDimensionPage     = "PAGE"
)

type SearchConsoleCmd struct {
	Sites           SearchConsoleSitesCmd           `cmd:"" name:"sites" aliases:"list,ls" help:"List and inspect Search Console sites"`
	SearchAnalytics SearchConsoleSearchAnalyticsCmd `cmd:"" name:"searchanalytics" aliases:"analytics" help:"Search Analytics queries"`
	Query           SearchConsoleQueryCmd           `cmd:"" name:"query" aliases:"report" help:"Run a Search Analytics query"`
	Sitemaps        SearchConsoleSitemapsCmd        `cmd:"" name:"sitemaps" help:"List/get/submit/delete sitemaps"`
}

type SearchConsoleSitesCmd struct {
	List SearchConsoleSitesListCmd `cmd:"" default:"withargs" aliases:"ls" help:"List accessible Search Console sites"`
	Get  SearchConsoleSitesGetCmd  `cmd:"" name:"get" aliases:"info,show" help:"Get a specific Search Console site"`
}

type SearchConsoleSitesListCmd struct {
	FailEmpty bool `name:"fail-empty" aliases:"non-empty,require-results" help:"Exit with code 3 if no results"`
}

func (c *SearchConsoleSitesListCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	resp, err := svc.Sites.List().Context(ctx).Do()
	if err != nil {
		return wrapSearchConsoleError(err)
	}

	rows := resp.SiteEntry
	if outfmt.IsJSON(ctx) {
		if err := outfmt.WriteJSON(ctx, os.Stdout, map[string]any{
			"sites": rows,
		}); err != nil {
			return err
		}
		if len(rows) == 0 {
			return failEmptyExit(c.FailEmpty)
		}
		return nil
	}

	if len(rows) == 0 {
		u.Err().Println("No Search Console sites")
		return failEmptyExit(c.FailEmpty)
	}

	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, "SITE\tPERMISSION")
	for _, item := range rows {
		if item == nil {
			continue
		}
		fmt.Fprintf(w, "%s\t%s\n", sanitizeTab(item.SiteUrl), sanitizeTab(item.PermissionLevel))
	}
	return nil
}

type SearchConsoleSitesGetCmd struct {
	SiteURL string `arg:"" name:"siteUrl" help:"Search Console property URL (e.g. https://example.com/ or sc-domain:example.com)"`
}

func (c *SearchConsoleSitesGetCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	siteURL := strings.TrimSpace(c.SiteURL)
	if siteURL == "" {
		return usage("empty siteUrl")
	}

	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	site, err := svc.Sites.Get(siteURL).Context(ctx).Do()
	if err != nil {
		return wrapSearchConsoleError(err)
	}

	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{
			"site": site,
		})
	}

	return writeResult(ctx, u,
		kv("site_url", site.SiteUrl),
		kv("permission_level", site.PermissionLevel),
	)
}

type SearchConsoleSearchAnalyticsCmd struct {
	Query SearchConsoleQueryCmd `cmd:"" name:"query" default:"withargs" aliases:"run" help:"Run a Search Analytics query"`
}

type SearchConsoleQueryCmd struct {
	SiteURL string `arg:"" name:"siteUrl" help:"Search Console property URL (e.g. https://example.com/ or sc-domain:example.com)"`

	From        string   `name:"from" aliases:"start" help:"Start date (YYYY-MM-DD)"`
	To          string   `name:"to" aliases:"end" help:"End date (YYYY-MM-DD)"`
	Dimensions  string   `name:"dimensions" help:"Comma-separated dimensions (DATE,QUERY,PAGE,COUNTRY,DEVICE,SEARCH_APPEARANCE,HOUR)" default:"QUERY"`
	Type        string   `name:"type" help:"Search type (WEB,IMAGE,VIDEO,NEWS,DISCOVER,GOOGLE_NEWS)" default:"WEB"`
	Aggregation string   `name:"aggregation" help:"Aggregation type (AUTO,BY_PROPERTY,BY_PAGE,BY_NEWS_SHOWCASE_PANEL)"`
	DataState   string   `name:"data-state" help:"Data state (FINAL,ALL,HOURLY_ALL)"`
	Max         int64    `name:"max" aliases:"limit" help:"Max rows to return (1-25000)" default:"1000"`
	Offset      int64    `name:"offset" aliases:"start-row" help:"Row offset for pagination" default:"0"`
	Filter      []string `name:"filter" help:"Dimension filter, repeatable: dimension:operator:expression"`
	Request     string   `name:"request" help:"SearchAnalyticsQueryRequest JSON spec. Accepts @file, a plain file path, -, or inline JSON."`
	FailEmpty   bool     `name:"fail-empty" aliases:"non-empty,require-results" help:"Exit with code 3 if no rows"`
}

func (c *SearchConsoleQueryCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	siteURL := strings.TrimSpace(c.SiteURL)
	if siteURL == "" {
		return usage("empty siteUrl")
	}

	req, err := c.buildRequest()
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	resp, err := svc.Searchanalytics.Query(siteURL, req).Context(ctx).Do()
	if err != nil {
		return wrapSearchConsoleError(err)
	}

	queryType := req.Type
	if queryType == "" {
		queryType = req.SearchType
	}

	if outfmt.IsJSON(ctx) {
		if err := outfmt.WriteJSON(ctx, os.Stdout, map[string]any{
			"site_url":                  siteURL,
			"from":                      req.StartDate,
			"to":                        req.EndDate,
			"type":                      queryType,
			"dimensions":                req.Dimensions,
			"response_aggregation_type": resp.ResponseAggregationType,
			"rows":                      resp.Rows,
		}); err != nil {
			return err
		}
		if len(resp.Rows) == 0 {
			return failEmptyExit(c.FailEmpty)
		}
		return nil
	}

	if len(resp.Rows) == 0 {
		u.Err().Println("No Search Console rows")
		return failEmptyExit(c.FailEmpty)
	}

	headers := requestSearchConsoleDimensions(req, resp.Rows)
	headers = append(headers, "CLICKS", "IMPRESSIONS", "CTR", "POSITION")

	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, strings.Join(headers, "\t"))
	for _, row := range resp.Rows {
		if row == nil {
			continue
		}
		values := make([]string, 0, len(headers))
		for i := range headers[:len(headers)-4] {
			values = append(values, sanitizeTab(searchConsoleKey(row, i)))
		}
		values = append(values,
			formatSearchConsoleMetric(row.Clicks, 0),
			formatSearchConsoleMetric(row.Impressions, 0),
			formatSearchConsoleMetric(row.Ctr, 4),
			formatSearchConsoleMetric(row.Position, 2),
		)
		fmt.Fprintln(w, strings.Join(values, "\t"))
	}
	return nil
}

func (c *SearchConsoleQueryCmd) buildRequest() (*searchconsoleapi.SearchAnalyticsQueryRequest, error) {
	requestSpec := strings.TrimSpace(c.Request)
	if requestSpec != "" {
		return buildSearchConsoleRequestFromSpec(requestSpec)
	}

	from, err := parseSearchConsoleDate(c.From, "--from")
	if err != nil {
		return nil, err
	}
	to, err := parseSearchConsoleDate(c.To, "--to")
	if err != nil {
		return nil, err
	}
	if rangeErr := validateSearchConsoleDateRange(from, to); rangeErr != nil {
		return nil, rangeErr
	}

	if c.Max <= 0 || c.Max > maxSearchConsoleRowLimit {
		return nil, usagef("--max must be between 1 and %d", maxSearchConsoleRowLimit)
	}
	if c.Offset < 0 {
		return nil, usage("--offset must be >= 0")
	}

	dimensions, err := normalizeSearchConsoleDimensions(c.Dimensions)
	if err != nil {
		return nil, err
	}
	searchType, err := normalizeSearchConsoleType(c.Type)
	if err != nil {
		return nil, err
	}

	req := &searchconsoleapi.SearchAnalyticsQueryRequest{
		StartDate:  from,
		EndDate:    to,
		Dimensions: dimensions,
		Type:       searchType,
		RowLimit:   c.Max,
		StartRow:   c.Offset,
	}

	if v := strings.TrimSpace(c.Aggregation); v != "" {
		aggregation, err := normalizeSearchConsoleAggregation(v)
		if err != nil {
			return nil, err
		}
		req.AggregationType = aggregation
	}
	if v := strings.TrimSpace(c.DataState); v != "" {
		dataState, err := normalizeSearchConsoleDataState(v)
		if err != nil {
			return nil, err
		}
		req.DataState = dataState
	}

	if len(c.Filter) > 0 {
		filters := make([]*searchconsoleapi.ApiDimensionFilter, 0, len(c.Filter))
		for _, raw := range c.Filter {
			filter, err := parseSearchConsoleFilter(raw)
			if err != nil {
				return nil, err
			}
			filters = append(filters, filter)
		}
		req.DimensionFilterGroups = []*searchconsoleapi.ApiDimensionFilterGroup{
			{
				GroupType: searchConsoleGroupAnd,
				Filters:   filters,
			},
		}
	}

	return req, nil
}

type SearchConsoleSitemapsCmd struct {
	List   SearchConsoleSitemapsListCmd   `cmd:"" default:"withargs" aliases:"ls" help:"List sitemaps for a site"`
	Get    SearchConsoleSitemapsGetCmd    `cmd:"" name:"get" aliases:"info,show" help:"Get a sitemap"`
	Submit SearchConsoleSitemapsSubmitCmd `cmd:"" name:"submit" help:"Submit a sitemap"`
	Delete SearchConsoleSitemapsDeleteCmd `cmd:"" name:"delete" aliases:"rm,remove" help:"Delete a sitemap"`
}

type SearchConsoleSitemapsListCmd struct {
	SiteURL      string `arg:"" name:"siteUrl" help:"Search Console property URL (e.g. https://example.com/ or sc-domain:example.com)"`
	SitemapIndex string `name:"sitemap-index" help:"Filter to a sitemap index URL"`
	FailEmpty    bool   `name:"fail-empty" aliases:"non-empty,require-results" help:"Exit with code 3 if no results"`
}

func (c *SearchConsoleSitemapsListCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	siteURL := strings.TrimSpace(c.SiteURL)
	if siteURL == "" {
		return usage("empty siteUrl")
	}

	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	call := svc.Sitemaps.List(siteURL).Context(ctx)
	if v := strings.TrimSpace(c.SitemapIndex); v != "" {
		call = call.SitemapIndex(v)
	}
	resp, err := call.Do()
	if err != nil {
		return wrapSearchConsoleError(err)
	}

	rows := resp.Sitemap
	if outfmt.IsJSON(ctx) {
		if err := outfmt.WriteJSON(ctx, os.Stdout, map[string]any{
			"sitemaps": rows,
		}); err != nil {
			return err
		}
		if len(rows) == 0 {
			return failEmptyExit(c.FailEmpty)
		}
		return nil
	}

	if len(rows) == 0 {
		u.Err().Println("No Search Console sitemaps")
		return failEmptyExit(c.FailEmpty)
	}

	w, flush := tableWriter(ctx)
	defer flush()
	fmt.Fprintln(w, "PATH\tTYPE\tPENDING\tWARNINGS\tERRORS\tLAST_SUBMITTED\tLAST_DOWNLOADED\tCONTENTS")
	for _, item := range rows {
		if item == nil {
			continue
		}
		fmt.Fprintf(w, "%s\t%s\t%t\t%d\t%d\t%s\t%s\t%s\n",
			sanitizeTab(item.Path),
			sanitizeTab(item.Type),
			item.IsPending,
			item.Warnings,
			item.Errors,
			sanitizeTab(item.LastSubmitted),
			sanitizeTab(item.LastDownloaded),
			sanitizeTab(formatSearchConsoleSitemapContents(item.Contents)),
		)
	}
	return nil
}

type SearchConsoleSitemapsGetCmd struct {
	SiteURL  string `arg:"" name:"siteUrl" help:"Search Console property URL (e.g. https://example.com/ or sc-domain:example.com)"`
	FeedPath string `arg:"" name:"feedpath" help:"Sitemap URL"`
}

func (c *SearchConsoleSitemapsGetCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	siteURL := strings.TrimSpace(c.SiteURL)
	if siteURL == "" {
		return usage("empty siteUrl")
	}
	feedPath := strings.TrimSpace(c.FeedPath)
	if feedPath == "" {
		return usage("empty feedpath")
	}

	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	sitemap, err := svc.Sitemaps.Get(siteURL, feedPath).Context(ctx).Do()
	if err != nil {
		return wrapSearchConsoleError(err)
	}

	if outfmt.IsJSON(ctx) {
		return outfmt.WriteJSON(ctx, os.Stdout, map[string]any{
			"sitemap": sitemap,
		})
	}

	return writeResult(ctx, u,
		kv("path", sitemap.Path),
		kv("type", sitemap.Type),
		kv("pending", sitemap.IsPending),
		kv("warnings", sitemap.Warnings),
		kv("errors", sitemap.Errors),
		kv("last_submitted", sitemap.LastSubmitted),
		kv("last_downloaded", sitemap.LastDownloaded),
		kv("contents", formatSearchConsoleSitemapContents(sitemap.Contents)),
	)
}

type SearchConsoleSitemapsSubmitCmd struct {
	SiteURL  string `arg:"" name:"siteUrl" help:"Search Console property URL (e.g. https://example.com/ or sc-domain:example.com)"`
	FeedPath string `arg:"" name:"feedpath" help:"Sitemap URL"`
}

func (c *SearchConsoleSitemapsSubmitCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	siteURL := strings.TrimSpace(c.SiteURL)
	if siteURL == "" {
		return usage("empty siteUrl")
	}
	feedPath := strings.TrimSpace(c.FeedPath)
	if feedPath == "" {
		return usage("empty feedpath")
	}

	if err := dryRunExit(ctx, flags, "searchconsole.sitemaps.submit", map[string]any{
		"site_url":  siteURL,
		"feed_path": feedPath,
	}); err != nil {
		return err
	}

	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	if err := svc.Sitemaps.Submit(siteURL, feedPath).Context(ctx).Do(); err != nil {
		return wrapSearchConsoleError(err)
	}

	return writeResult(ctx, u,
		kv("submitted", true),
		kv("site_url", siteURL),
		kv("feed_path", feedPath),
	)
}

type SearchConsoleSitemapsDeleteCmd struct {
	SiteURL  string `arg:"" name:"siteUrl" help:"Search Console property URL (e.g. https://example.com/ or sc-domain:example.com)"`
	FeedPath string `arg:"" name:"feedpath" help:"Sitemap URL"`
}

func (c *SearchConsoleSitemapsDeleteCmd) Run(ctx context.Context, flags *RootFlags) error {
	u := ui.FromContext(ctx)
	siteURL := strings.TrimSpace(c.SiteURL)
	if siteURL == "" {
		return usage("empty siteUrl")
	}
	feedPath := strings.TrimSpace(c.FeedPath)
	if feedPath == "" {
		return usage("empty feedpath")
	}

	if err := dryRunAndConfirmDestructive(ctx, flags, "searchconsole.sitemaps.delete", map[string]any{
		"site_url":  siteURL,
		"feed_path": feedPath,
	}, fmt.Sprintf("delete sitemap %s", feedPath)); err != nil {
		return err
	}

	account, err := requireAccount(flags)
	if err != nil {
		return err
	}

	svc, err := newSearchConsoleService(ctx, account)
	if err != nil {
		return err
	}
	if err := svc.Sitemaps.Delete(siteURL, feedPath).Context(ctx).Do(); err != nil {
		return wrapSearchConsoleError(err)
	}

	return writeResult(ctx, u,
		kv("deleted", true),
		kv("site_url", siteURL),
		kv("feed_path", feedPath),
	)
}

func buildSearchConsoleRequestFromSpec(spec string) (*searchconsoleapi.SearchAnalyticsQueryRequest, error) {
	b, err := readSearchConsoleRequestBytes(spec)
	if err != nil {
		return nil, err
	}

	var req searchconsoleapi.SearchAnalyticsQueryRequest
	if unmarshalErr := json.Unmarshal(b, &req); unmarshalErr != nil {
		return nil, fmt.Errorf("decode search console request: %w", unmarshalErr)
	}

	if req.RowLimit == 0 {
		req.RowLimit = defaultSearchConsoleRowLimit
	}
	if req.RowLimit < 1 || req.RowLimit > maxSearchConsoleRowLimit {
		return nil, usagef("request.rowLimit must be between 1 and %d", maxSearchConsoleRowLimit)
	}
	if req.StartRow < 0 {
		return nil, usage("request.startRow must be >= 0")
	}
	if rangeErr := validateSearchConsoleDateRange(req.StartDate, req.EndDate); rangeErr != nil {
		return nil, rangeErr
	}

	if len(req.Dimensions) > 0 {
		dimensions, dimErr := normalizeSearchConsoleDimensionList(req.Dimensions)
		if dimErr != nil {
			return nil, dimErr
		}
		req.Dimensions = dimensions
	}

	if req.Type == "" && req.SearchType != "" {
		req.Type = req.SearchType
	}
	if req.Type == "" {
		req.Type = searchConsoleTypeWeb
	}
	searchType, err := normalizeSearchConsoleType(req.Type)
	if err != nil {
		return nil, err
	}
	req.Type = searchType
	req.SearchType = searchType

	if v := strings.TrimSpace(req.AggregationType); v != "" {
		aggregation, err := normalizeSearchConsoleAggregation(v)
		if err != nil {
			return nil, err
		}
		req.AggregationType = aggregation
	}
	if v := strings.TrimSpace(req.DataState); v != "" {
		dataState, err := normalizeSearchConsoleDataState(v)
		if err != nil {
			return nil, err
		}
		req.DataState = dataState
	}

	for _, group := range req.DimensionFilterGroups {
		if group == nil {
			continue
		}
		if strings.TrimSpace(group.GroupType) == "" {
			group.GroupType = searchConsoleGroupAnd
		}
		if !strings.EqualFold(strings.TrimSpace(group.GroupType), "and") {
			return nil, usagef("invalid request.groupType %q (expected and)", group.GroupType)
		}
		group.GroupType = searchConsoleGroupAnd
		for _, filter := range group.Filters {
			if filter == nil {
				continue
			}
			dimension, err := normalizeSearchConsoleDimension(filter.Dimension)
			if err != nil {
				return nil, err
			}
			operator, err := normalizeSearchConsoleFilterOperator(filter.Operator)
			if err != nil {
				return nil, err
			}
			if strings.TrimSpace(filter.Expression) == "" {
				return nil, usage("request filter expression cannot be empty")
			}
			filter.Dimension = dimension
			filter.Operator = operator
		}
	}

	return &req, nil
}

func readSearchConsoleRequestBytes(spec string) ([]byte, error) {
	spec = strings.TrimSpace(spec)
	switch {
	case spec == "", spec == "-", strings.HasPrefix(spec, "@"), strings.HasPrefix(spec, "{"), strings.HasPrefix(spec, "["):
		return resolveInlineOrFileBytes(spec)
	default:
		path, err := config.ExpandPath(spec)
		if err != nil {
			return nil, err
		}
		return os.ReadFile(path) //nolint:gosec // user-provided path
	}
}

func parseSearchConsoleDate(value string, flagName string) (string, error) {
	value = strings.TrimSpace(value)
	if value == "" {
		return "", usagef("empty %s", flagName)
	}
	if _, err := time.Parse("2006-01-02", value); err != nil {
		return "", usagef("invalid %s (expected YYYY-MM-DD)", flagName)
	}
	return value, nil
}

func validateSearchConsoleDateRange(from string, to string) error {
	start, err := time.Parse("2006-01-02", strings.TrimSpace(from))
	if err != nil {
		return usage("invalid start date (expected YYYY-MM-DD)")
	}
	end, err := time.Parse("2006-01-02", strings.TrimSpace(to))
	if err != nil {
		return usage("invalid end date (expected YYYY-MM-DD)")
	}
	if end.Before(start) {
		return usage("--to must be on or after --from")
	}
	return nil
}

func normalizeSearchConsoleType(raw string) (string, error) {
	v := strings.TrimSpace(raw)
	switch {
	case strings.EqualFold(v, "web"):
		return searchConsoleTypeWeb, nil
	case strings.EqualFold(v, "image"):
		return "IMAGE", nil
	case strings.EqualFold(v, "video"):
		return "VIDEO", nil
	case strings.EqualFold(v, "news"):
		return "NEWS", nil
	case strings.EqualFold(v, "discover"):
		return "DISCOVER", nil
	case strings.EqualFold(strings.ReplaceAll(v, "_", ""), "googleNews"),
		strings.EqualFold(strings.ReplaceAll(v, "-", ""), "googleNews"):
		return "GOOGLE_NEWS", nil
	case v == "":
		return "", usage("empty --type")
	default:
		return "", usagef("invalid --type %q (expected WEB|IMAGE|VIDEO|NEWS|DISCOVER|GOOGLE_NEWS)", raw)
	}
}

func normalizeSearchConsoleAggregation(raw string) (string, error) {
	v := strings.TrimSpace(raw)
	switch {
	case v == "":
		return "", nil
	case strings.EqualFold(v, "auto"):
		return "AUTO", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "byProperty"):
		return "BY_PROPERTY", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "byPage"):
		return searchConsoleAggregationByPage, nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "byNewsShowcasePanel"):
		return "BY_NEWS_SHOWCASE_PANEL", nil
	default:
		return "", usagef("invalid --aggregation %q (expected AUTO|BY_PROPERTY|BY_PAGE|BY_NEWS_SHOWCASE_PANEL)", raw)
	}
}

func normalizeSearchConsoleDataState(raw string) (string, error) {
	v := strings.TrimSpace(raw)
	switch {
	case v == "":
		return "", nil
	case strings.EqualFold(v, "final"):
		return "FINAL", nil
	case strings.EqualFold(v, "all"):
		return "ALL", nil
	case strings.EqualFold(strings.ReplaceAll(v, "-", "_"), "hourly_all"):
		return "HOURLY_ALL", nil
	default:
		return "", usagef("invalid --data-state %q (expected FINAL|ALL|HOURLY_ALL)", raw)
	}
}

func normalizeSearchConsoleDimensions(raw string) ([]string, error) {
	parts := splitCommaList(raw)
	if len(parts) == 0 {
		return nil, nil
	}
	return normalizeSearchConsoleDimensionList(parts)
}

func normalizeSearchConsoleDimensionList(parts []string) ([]string, error) {
	out := make([]string, 0, len(parts))
	for _, part := range parts {
		v, err := normalizeSearchConsoleDimension(part)
		if err != nil {
			return nil, err
		}
		out = append(out, v)
	}
	return out, nil
}

func normalizeSearchConsoleDimension(raw string) (string, error) {
	v := strings.TrimSpace(raw)
	switch {
	case strings.EqualFold(v, "date"):
		return "DATE", nil
	case strings.EqualFold(v, "query"):
		return searchConsoleDimensionQuery, nil
	case strings.EqualFold(v, "page"):
		return searchConsoleDimensionPage, nil
	case strings.EqualFold(v, "country"):
		return "COUNTRY", nil
	case strings.EqualFold(v, "device"):
		return "DEVICE", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "searchAppearance"):
		return "SEARCH_APPEARANCE", nil
	case strings.EqualFold(v, "hour"):
		return "HOUR", nil
	case v == "":
		return "", usage("empty dimension")
	default:
		return "", usagef("invalid dimension %q (expected DATE|QUERY|PAGE|COUNTRY|DEVICE|SEARCH_APPEARANCE|HOUR)", raw)
	}
}

func parseSearchConsoleFilter(raw string) (*searchconsoleapi.ApiDimensionFilter, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return nil, usage("empty --filter")
	}

	first := strings.Index(raw, ":")
	if first <= 0 {
		return nil, usagef("invalid --filter %q (expected dimension:operator:expression)", raw)
	}
	rest := raw[first+1:]
	second := strings.Index(rest, ":")
	if second < 0 {
		return nil, usagef("invalid --filter %q (expected dimension:operator:expression)", raw)
	}

	dimension, err := normalizeSearchConsoleDimension(raw[:first])
	if err != nil {
		return nil, err
	}
	operator, err := normalizeSearchConsoleFilterOperator(rest[:second])
	if err != nil {
		return nil, err
	}
	expression := strings.TrimSpace(rest[second+1:])
	if expression == "" {
		return nil, usagef("invalid --filter %q (expected dimension:operator:expression)", raw)
	}

	return &searchconsoleapi.ApiDimensionFilter{
		Dimension:  dimension,
		Operator:   operator,
		Expression: expression,
	}, nil
}

func normalizeSearchConsoleFilterOperator(raw string) (string, error) {
	v := strings.TrimSpace(raw)
	switch {
	case v == "":
		return "", usage("empty filter operator")
	case strings.EqualFold(v, "equals"):
		return "EQUALS", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "notEquals"):
		return "NOT_EQUALS", nil
	case strings.EqualFold(v, "contains"):
		return "CONTAINS", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "notContains"):
		return "NOT_CONTAINS", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "includingRegex"):
		return "INCLUDING_REGEX", nil
	case strings.EqualFold(strings.ReplaceAll(strings.ReplaceAll(v, "_", ""), "-", ""), "excludingRegex"):
		return "EXCLUDING_REGEX", nil
	default:
		return "", usagef("invalid filter operator %q (expected EQUALS|NOT_EQUALS|CONTAINS|NOT_CONTAINS|INCLUDING_REGEX|EXCLUDING_REGEX)", raw)
	}
}

func requestSearchConsoleDimensions(
	req *searchconsoleapi.SearchAnalyticsQueryRequest,
	rows []*searchconsoleapi.ApiDataRow,
) []string {
	if req != nil && len(req.Dimensions) > 0 {
		out := make([]string, 0, len(req.Dimensions))
		for _, dim := range req.Dimensions {
			out = append(out, strings.ToUpper(strings.TrimSpace(dim)))
		}
		return out
	}

	keyCount := 0
	for _, row := range rows {
		if row != nil && len(row.Keys) > keyCount {
			keyCount = len(row.Keys)
		}
	}

	out := make([]string, 0, keyCount)
	for i := 0; i < keyCount; i++ {
		out = append(out, "KEY_"+strconv.Itoa(i+1))
	}
	return out
}

func searchConsoleKey(row *searchconsoleapi.ApiDataRow, index int) string {
	if row == nil || index < 0 || index >= len(row.Keys) {
		return ""
	}
	return row.Keys[index]
}

func formatSearchConsoleMetric(v float64, decimals int) string {
	if decimals <= 0 {
		return strconv.FormatFloat(v, 'f', 0, 64)
	}
	return strconv.FormatFloat(v, 'f', decimals, 64)
}

func formatSearchConsoleSitemapContents(contents []*searchconsoleapi.WmxSitemapContent) string {
	if len(contents) == 0 {
		return ""
	}

	parts := make([]string, 0, len(contents))
	for _, content := range contents {
		if content == nil {
			continue
		}
		parts = append(parts, fmt.Sprintf("%s:%d/%d", content.Type, content.Indexed, content.Submitted))
	}
	return strings.Join(parts, ",")
}

func wrapSearchConsoleError(err error) error {
	var apiErr *gapi.Error
	if !errors.As(err, &apiErr) {
		return err
	}
	if apiErr.Code != 403 {
		return err
	}

	message := strings.ToLower(apiErr.Message)
	switch {
	case strings.Contains(message, "accessnotconfigured"), strings.Contains(message, "api has not been used"):
		return fmt.Errorf("search console API is not enabled for this OAuth project. Enable it at https://console.cloud.google.com/apis/api/searchconsole.googleapis.com")
	case strings.Contains(message, "insufficientpermissions"), strings.Contains(message, "insufficient permission"):
		return fmt.Errorf("insufficient permissions for Search Console API. Re-authorize with: gog auth add <email> --services searchconsole")
	default:
		return err
	}
}
