package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/steipete/gogcli/internal/googleapi"
	"golang.org/x/oauth2"
	"golang.org/x/oauth2/google"
	"google.golang.org/api/gmail/v1"
	"google.golang.org/api/option"
	"google.golang.org/genai"
)

const cleanSystemInstruction = `You are an AI email assistant. Your task is to analyze email thread summaries and classify each thread into one of three actions:
- TRASH: for spam, marketing, newsletters, promotions, automated alerts that are no longer useful, or low-value emails.
- ARCHIVE: for receipts, transaction confirmations, shipping notifications, updates that should be saved for reference but do not need response or attention in the inbox.
- KEEP: for personal emails, direct messages from real people, work discussions, or important emails requiring user attention.

You must output a JSON array of objects. Each object must have:
- "id": the thread ID.
- "action": either "TRASH", "ARCHIVE", or "KEEP".
- "reason": a short explanation of why you chose this action (max 10 words).

Output ONLY the raw JSON array. Do not wrap in markdown or any other formatting.`

type ThreadSummary struct {
	ID      string `json:"id"`
	Subject string `json:"subject"`
	From    string `json:"from"`
	Date    string `json:"date"`
	Snippet string `json:"snippet"`
}

type ClassificationResult struct {
	ID     string `json:"id"`
	Action string `json:"action"` // "TRASH", "ARCHIVE", "KEEP"
	Reason string `json:"reason"`
}

type GmailCleanResponse struct {
	Threads []ClassificationResult `json:"threads"`
}

func newGmailServiceForClean(ctx context.Context, accountEmail string) (*gmail.Service, error) {
	scopes := []string{"https://www.googleapis.com/auth/gmail.modify"}

	// Try loading existing gcloud legacy credentials for this user
	if accountEmail != "" && accountEmail != "adc" {
		appData := os.Getenv("APPDATA")
		if appData != "" {
			gcloudCredPath := filepath.Join(appData, "gcloud", "legacy_credentials", accountEmail, "adc.json")
			if _, err := os.Stat(gcloudCredPath); err == nil {
				data, err := os.ReadFile(gcloudCredPath)
				if err == nil {
					var creds struct {
						ClientID     string `json:"client_id"`
						ClientSecret string `json:"client_secret"`
						RefreshToken string `json:"refresh_token"`
					}
					if err := json.Unmarshal(data, &creds); err == nil && creds.RefreshToken != "" {
						cfg := oauth2.Config{
							ClientID:     creds.ClientID,
							ClientSecret: creds.ClientSecret,
							Endpoint:     google.Endpoint,
							Scopes:       scopes,
						}
						ts := cfg.TokenSource(ctx, &oauth2.Token{
							RefreshToken: creds.RefreshToken,
						})
						svc, err := gmail.NewService(ctx, option.WithTokenSource(ts))
						if err == nil {
							return svc, nil
						}
					}
				}
			}
		}
	}

	// If GOG_AUTH_MODE=adc, and we have a specific email address (not empty, and not "adc"),
	// and we have GOOGLE_APPLICATION_CREDENTIALS, we can perform user impersonation (domain-wide delegation).
	if googleapi.IsADCMode() && accountEmail != "" && accountEmail != "adc" {
		credPath := os.Getenv("GOOGLE_APPLICATION_CREDENTIALS")
		if credPath != "" {
			data, err := os.ReadFile(credPath)
			if err == nil {
				config, err := google.JWTConfigFromJSON(data, scopes...)
				if err == nil {
					config.Subject = accountEmail
					svc, err := gmail.NewService(ctx, option.WithTokenSource(config.TokenSource(ctx)))
					if err == nil {
						return svc, nil
					}
				}
			}
		}
	}

	// Fallback to standard gogcli service creation
	return googleapi.NewGmail(ctx, accountEmail)
}

func runGmailClean(ctx context.Context, req Request) (Response, error) {
	accountEmail := req.AccountEmail
	if accountEmail == "" {
		accountEmail = req.Text
	}
	if accountEmail == "" {
		accountEmail = os.Getenv("GOG_ACCOUNT")
	}
	if accountEmail == "" {
		if googleapi.IsADCMode() {
			accountEmail = "adc"
		} else {
			return Response{}, errors.New("account email is required")
		}
	}

	maxThreads := req.MaxMessages
	if maxThreads <= 0 {
		maxThreads = 50
	}

	dryRun := req.DryRun

	fmt.Fprintf(os.Stderr, "Initializing Gmail service for account: %s...\n", accountEmail)
	svc, err := newGmailServiceForClean(ctx, accountEmail)
	if err != nil {
		return Response{}, fmt.Errorf("failed to initialize Gmail service: %w", err)
	}

	fmt.Fprintf(os.Stderr, "Scanning inbox (max %d threads)...\n", maxThreads)
	listCall := svc.Users.Threads.List("me").Q("label:inbox")
	if maxThreads > 0 {
		listCall = listCall.MaxResults(int64(maxThreads))
	}
	res, err := listCall.Context(ctx).Do()
	if err != nil {
		return Response{}, fmt.Errorf("failed to list inbox threads: %w", err)
	}

	if len(res.Threads) == 0 {
		fmt.Fprintln(os.Stderr, "Inbox is clean. No threads found.")
		return Response{
			Text: `{"threads":[]}`,
		}, nil
	}

	fmt.Fprintf(os.Stderr, "Found %d threads. Fetching thread metadata...\n", len(res.Threads))
	var summaries []ThreadSummary
	for i, t := range res.Threads {
		fmt.Fprintf(os.Stderr, "[%d/%d] Fetching thread %s...\n", i+1, len(res.Threads), t.Id)
		thread, err := svc.Users.Threads.Get("me", t.Id).Format("metadata").MetadataHeaders("From", "Subject", "Date").Context(ctx).Do()
		if err != nil {
			fmt.Fprintf(os.Stderr, "Warning: failed to fetch thread %s: %v\n", t.Id, err)
			continue
		}

		var subject, from, date, snippet string
		if len(thread.Messages) > 0 {
			snippet = thread.Snippet
			if snippet == "" {
				snippet = thread.Messages[len(thread.Messages)-1].Snippet
			}

			// Extract Subject from first message with a Subject
			for _, msg := range thread.Messages {
				if msg.Payload != nil {
					for _, h := range msg.Payload.Headers {
						if strings.EqualFold(h.Name, "Subject") && subject == "" {
							subject = h.Value
						}
						if strings.EqualFold(h.Name, "From") {
							from = h.Value
						}
						if strings.EqualFold(h.Name, "Date") {
							date = h.Value
						}
					}
				}
			}
		}

		summaries = append(summaries, ThreadSummary{
			ID:      t.Id,
			Subject: subject,
			From:    from,
			Date:    date,
			Snippet: snippet,
		})
	}

	if len(summaries) == 0 {
		return Response{
			Text: `{"threads":[]}`,
		}, nil
	}

	fmt.Fprintln(os.Stderr, "Classifying threads with Gemini 2.5...")
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	summariesJSON, err := json.Marshal(summaries)
	if err != nil {
		return Response{}, fmt.Errorf("failed to marshal thread summaries: %w", err)
	}

	prompt := fmt.Sprintf("Classify the following email threads:\n\n%s", string(summariesJSON))
	contents := []*genai.Content{
		{
			Parts: []*genai.Part{genai.NewPartFromText(prompt)},
			Role:  "user",
		},
	}

	config := &genai.GenerateContentConfig{
		SystemInstruction: &genai.Content{
			Parts: []*genai.Part{genai.NewPartFromText(cleanSystemInstruction)},
			Role:  "system",
		},
		ResponseMIMEType: "application/json",
	}

	resp, err := client.Models.GenerateContent(ctx, "gemini-2.5-flash", contents, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to classify with Gemini: %w", err)
	}

	rawText := cleanJSONResponse(resp.Text())
	var classifications []ClassificationResult
	if err := json.Unmarshal([]byte(rawText), &classifications); err != nil {
		return Response{}, fmt.Errorf("failed to parse Gemini classifications: %w (raw response: %s)", err, rawText)
	}

	fmt.Fprintf(os.Stderr, "Applying actions (dry-run=%t)...\n", dryRun)
	var processed []ClassificationResult
	for _, cls := range classifications {
		action := strings.ToUpper(strings.TrimSpace(cls.Action))
		fmt.Fprintf(os.Stderr, "- Thread %s: action=%s, reason=%q\n", cls.ID, action, cls.Reason)

		if dryRun {
			processed = append(processed, cls)
			continue
		}

		switch action {
		case "TRASH":
			_, err := svc.Users.Threads.Trash("me", cls.ID).Context(ctx).Do()
			if err != nil {
				fmt.Fprintf(os.Stderr, "  Error trashing thread %s: %v\n", cls.ID, err)
			} else {
				processed = append(processed, cls)
			}
		case "ARCHIVE":
			modifyReq := &gmail.ModifyThreadRequest{
				RemoveLabelIds: []string{"INBOX"},
			}
			_, err := svc.Users.Threads.Modify("me", cls.ID, modifyReq).Context(ctx).Do()
			if err != nil {
				fmt.Fprintf(os.Stderr, "  Error archiving thread %s: %v\n", cls.ID, err)
			} else {
				processed = append(processed, cls)
			}
		case "KEEP":
			// Keep in inbox, no action
			processed = append(processed, cls)
		default:
			fmt.Fprintf(os.Stderr, "  Unknown action %q for thread %s\n", action, cls.ID)
		}
	}

	responseBytes, err := json.Marshal(GmailCleanResponse{Threads: processed})
	if err != nil {
		return Response{}, fmt.Errorf("failed to marshal clean response: %w", err)
	}

	return Response{
		Text: string(responseBytes),
	}, nil
}

func cleanJSONResponse(raw string) string {
	raw = strings.TrimSpace(raw)
	if strings.HasPrefix(raw, "```json") {
		raw = strings.TrimPrefix(raw, "```json")
		raw = strings.TrimSuffix(raw, "```")
	} else if strings.HasPrefix(raw, "```") {
		raw = strings.TrimPrefix(raw, "```")
		raw = strings.TrimSuffix(raw, "```")
	}
	return strings.TrimSpace(raw)
}
