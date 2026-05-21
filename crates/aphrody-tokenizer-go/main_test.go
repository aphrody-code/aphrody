// SPDX-License-Identifier: Apache-2.0
package main

import (
	"encoding/json"
	"strings"
	"testing"
	"time"

	"github.com/tiktoken-go/tokenizer"
	"google.golang.org/genai"
)

func TestGetEncoding(t *testing.T) {
	tests := []struct {
		input    string
		expected tokenizer.Encoding
		wantErr  bool
	}{
		{"cl100k_base", tokenizer.Cl100kBase, false},
		{"CL100K", tokenizer.Cl100kBase, false},
		{"o200k_base", tokenizer.O200kBase, false},
		{"o200k", tokenizer.O200kBase, false},
		{"p50k_base", tokenizer.P50kBase, false},
		{"p50k", tokenizer.P50kBase, false},
		{"r50k_base", tokenizer.R50kBase, false},
		{"gpt2", tokenizer.R50kBase, false},
		{"invalid", "", true},
	}

	for _, tt := range tests {
		got, err := getEncoding(tt.input)
		if (err != nil) != tt.wantErr {
			t.Errorf("getEncoding(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
			continue
		}
		if got != tt.expected {
			t.Errorf("getEncoding(%q) = %q, want %q", tt.input, got, tt.expected)
		}
	}
}

func TestHTMLToText(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{
			name:     "simple paragraph",
			input:    "<p>Hello, World!</p>",
			expected: "Hello, World!",
		},
		{
			name:     "nested tags and headings",
			input:    "<h1>Title</h1><p>This is a <strong>bold</strong> statement.</p>",
			expected: "# Title\n\nThis is a bold statement.",
		},
		{
			name:     "exclude script and style",
			input:    "<div>Some content<script>alert(1)</script><style>body { color: red; }</style> and more content</div>",
			expected: "Some content and more content",
		},
		{
			name:     "list items",
			input:    "<ul><li>Item 1</li><li>Item 2</li></ul>",
			expected: "- Item 1\n- Item 2",
		},
		{
			name:     "links to markdown",
			input:    `<p>Check <a href="https://example.com">this link</a> out.</p>`,
			expected: "Check [this link](https://example.com) out.",
		},
		{
			name: "pre and code blocks",
			input: `<pre>func main() {
	println("hello")
}</pre>And some <code>inline code</code>.`,
			expected: "```\nfunc main() {\n\tprintln(\"hello\")\n}\n```\n\nAnd some `inline code`.",
		},
		{
			name:     "table elements",
			input:    "<table><tr><th>Header 1</th><th>Header 2</th></tr><tr><td>Value 1</td><td>Value 2</td></tr></table>",
			expected: "| Header 1 Header 2 |\n| Value 1 Value 2 |",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := htmlToText(tt.input)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}
			if got != tt.expected {
				t.Errorf("htmlToText() = %q, want %q", got, tt.expected)
			}
		})
	}
}

func TestJSONMarshaling(t *testing.T) {
	reqJSON := `{
		"command": "gemini",
		"model": "gemini-2.5-flash",
		"prompt": "Hello",
		"system_instruction": "Be helpful",
		"google_search": true,
		"response_mime_type": "application/json",
		"file_uri": "https://example.com/file",
		"file_mime_type": "image/png",
		"file_path": "path/to/file",
		"display_name": "test file",
		"aspect_ratio": "16:9",
		"number_of_images": 2,
		"output_mime_type": "image/png",
		"output_path": "out.png"
	}`

	var req Request
	err := json.Unmarshal([]byte(reqJSON), &req)
	if err != nil {
		t.Fatalf("Failed to unmarshal Request: %v", err)
	}

	if req.Command != "gemini" {
		t.Errorf("expected command 'gemini', got %q", req.Command)
	}
	if req.Model != "gemini-2.5-flash" {
		t.Errorf("expected model 'gemini-2.5-flash', got %q", req.Model)
	}
	if req.Prompt != "Hello" {
		t.Errorf("expected prompt 'Hello', got %q", req.Prompt)
	}
	if !req.GoogleSearch {
		t.Errorf("expected GoogleSearch true, got false")
	}

	resp := Response{
		Tokens:      10,
		Text:        "Generated text",
		URI:         "uri",
		Name:        "name",
		MIMEType:    "mime",
		TotalTokens: 12,
		Embedding:   []float32{0.1, 0.2, 0.3},
	}

	respBytes, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("Failed to marshal Response: %v", err)
	}

	var respMap map[string]interface{}
	err = json.Unmarshal(respBytes, &respMap)
	if err != nil {
		t.Fatalf("Failed to unmarshal Response bytes: %v", err)
	}

	if val, ok := respMap["total_tokens"]; !ok || int(val.(float64)) != 12 {
		t.Errorf("expected total_tokens to be 12, got %v", respMap["total_tokens"])
	}
	if val, ok := respMap["embedding"]; !ok {
		t.Errorf("expected embedding to be present")
	} else {
		vals := val.([]interface{})
		if len(vals) != 3 || vals[0].(float64) != 0.1 {
			t.Errorf("unexpected embedding values: %v", vals)
		}
	}
}

func TestMakeCreateCachedContentConfig(t *testing.T) {
	req := Request{
		DisplayName:       "My Cache",
		SystemInstruction: "You are a helpful assistant",
		TTLSeconds:        300,
		ExpireTime:        "2026-12-31T23:59:59Z",
	}

	config, err := makeCreateCachedContentConfig(req)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if config.DisplayName != "My Cache" {
		t.Errorf("expected DisplayName 'My Cache', got %q", config.DisplayName)
	}

	if config.SystemInstruction == nil || len(config.SystemInstruction.Parts) == 0 {
		t.Errorf("expected system instruction to be populated")
	}

	if config.TTL != 300*time.Second {
		t.Errorf("expected TTL to be 300s, got %v", config.TTL)
	}

	expectedTime, _ := time.Parse(time.RFC3339, "2026-12-31T23:59:59Z")
	if !config.ExpireTime.Equal(expectedTime) {
		t.Errorf("expected ExpireTime %v, got %v", expectedTime, config.ExpireTime)
	}
}

func TestModelsAndTuningsMarshaling(t *testing.T) {
	reqJSON := `{
		"command": "tune_model",
		"base_model": "models/gemini-2.5-flash",
		"tuning_dataset": {
			"gcsUri": "gs://my-bucket/training.jsonl"
		},
		"tuned_model_display_name": "My Tuned Model",
		"description": "Custom fine-tuned model",
		"epoch_count": 5,
		"learning_rate_multiplier": 0.5,
		"filter": "state=ACTIVE",
		"query_base": false
	}`

	var req Request
	err := json.Unmarshal([]byte(reqJSON), &req)
	if err != nil {
		t.Fatalf("Failed to unmarshal Request: %v", err)
	}

	if req.Command != "tune_model" {
		t.Errorf("expected command 'tune_model', got %q", req.Command)
	}
	if req.BaseModel != "models/gemini-2.5-flash" {
		t.Errorf("expected base_model 'models/gemini-2.5-flash', got %q", req.BaseModel)
	}
	if req.TuningDataset == nil || req.TuningDataset.GCSURI != "gs://my-bucket/training.jsonl" {
		t.Errorf("expected tuning_dataset GCSURI 'gs://my-bucket/training.jsonl'")
	}
	if req.TunedModelDisplayName != "My Tuned Model" {
		t.Errorf("expected tuned_model_display_name 'My Tuned Model', got %q", req.TunedModelDisplayName)
	}
	if req.Description != "Custom fine-tuned model" {
		t.Errorf("expected description 'Custom fine-tuned model', got %q", req.Description)
	}
	if req.EpochCount == nil || *req.EpochCount != 5 {
		t.Errorf("expected epoch_count 5")
	}
	if req.LearningRateMultiplier == nil || *req.LearningRateMultiplier != 0.5 {
		t.Errorf("expected learning_rate_multiplier 0.5")
	}
	if req.Filter != "state=ACTIVE" {
		t.Errorf("expected filter 'state=ACTIVE', got %q", req.Filter)
	}
	if req.QueryBase == nil || *req.QueryBase != false {
		t.Errorf("expected query_base to be false")
	}

	resp := Response{
		ModelInfo:      &genai.Model{Name: "models/gemini-2.5-flash"},
		ModelsList:     []*genai.Model{{Name: "models/gemini-2.5-flash"}},
		TuningJob:      &genai.TuningJob{Name: "tuningJobs/my-job"},
		TuningJobsList: []*genai.TuningJob{{Name: "tuningJobs/my-job"}},
	}

	respBytes, err := json.Marshal(resp)
	if err != nil {
		t.Fatalf("Failed to marshal Response: %v", err)
	}

	var respMap map[string]interface{}
	err = json.Unmarshal(respBytes, &respMap)
	if err != nil {
		t.Fatalf("Failed to unmarshal Response bytes: %v", err)
	}

	if modelInfo, ok := respMap["model_info"].(map[string]interface{}); !ok || modelInfo["name"] != "models/gemini-2.5-flash" {
		t.Errorf("expected model_info name 'models/gemini-2.5-flash', got %v", respMap["model_info"])
	}

	if modelsList, ok := respMap["models_list"].([]interface{}); !ok || len(modelsList) == 0 {
		t.Errorf("expected non-empty models_list")
	}

	if tuningJob, ok := respMap["tuning_job"].(map[string]interface{}); !ok || tuningJob["name"] != "tuningJobs/my-job" {
		t.Errorf("expected tuning_job name 'tuningJobs/my-job', got %v", respMap["tuning_job"])
	}

	if tuningJobsList, ok := respMap["tuning_jobs_list"].([]interface{}); !ok || len(tuningJobsList) == 0 {
		t.Errorf("expected non-empty tuning_jobs_list")
	}
}

func TestSessionCookieUnmarshal(t *testing.T) {
	// Test camelCase
	dataCamel := `{"name":"test1","value":"val1","httpOnly":true}`
	var sc1 SessionCookie
	if err := json.Unmarshal([]byte(dataCamel), &sc1); err != nil {
		t.Fatalf("failed to unmarshal camelCase: %v", err)
	}
	if !sc1.HttpOnly {
		t.Error("expected httpOnly to be true (camelCase)")
	}
	if sc1.Path != "/" {
		t.Errorf("expected default path '/', got %q", sc1.Path)
	}

	// Test snake_case
	dataSnake := `{"name":"test2","value":"val2","http_only":true,"path":"/my-path"}`
	var sc2 SessionCookie
	if err := json.Unmarshal([]byte(dataSnake), &sc2); err != nil {
		t.Fatalf("failed to unmarshal snake_case: %v", err)
	}
	if !sc2.HttpOnly {
		t.Error("expected httpOnly to be true (snake_case)")
	}
	if sc2.Path != "/my-path" {
		t.Errorf("expected path '/my-path', got %q", sc2.Path)
	}
}

func TestCookieJar(t *testing.T) {
	jar := NewCookieJar()
	jar.Insert(SessionCookie{Name: "B", Value: "2"})
	jar.Insert(SessionCookie{Name: "A", Value: "1"})

	expectedHeader := "A=1; B=2"
	if got := jar.HeaderValue(); got != expectedHeader {
		t.Errorf("expected header %q, got %q", expectedHeader, got)
	}

	if err := jar.RequireGoogleSession(); err == nil {
		t.Error("expected error for missing Google session cookies")
	}

	jar.Insert(SessionCookie{Name: "SAPISID", Value: "sapi"})
	jar.Insert(SessionCookie{Name: "__Secure-1PSID", Value: "psid"})
	if err := jar.RequireGoogleSession(); err != nil {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestParseTokensFromHTML(t *testing.T) {
	html := `<html>
	<script>
	window.WIZ_global_data = {
		"SNlM0e":"token_at",
		"cfb2h":"token_bl",
		"FdrFJe":"token_fsid"
	};
	</script>
	</html>`
	tokens, err := parseTokensFromHTML(html, "fr")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tokens.At != "token_at" || tokens.Bl != "token_bl" || tokens.Fsid != "token_fsid" || tokens.Language != "fr" {
		t.Errorf("unexpected parsed tokens: %+v", tokens)
	}

	// Test missing
	_, err = parseTokensFromHTML("invalid html", "en")
	if err == nil {
		t.Error("expected error for missing tokens")
	}
}

func TestGeminiModel(t *testing.T) {
	tests := []struct {
		modelName    string
		expectedModel GeminiModel
		expectedToken string
	}{
		{"flash-lite", ModelFlashLite, "1d44b34bcaa1c04d"},
		{"flash", ModelFlash, "56fdd199312815e2"},
		{"pro", ModelPro, "e6fa609c3fa255c0"},
		{"invalid", ModelFlash, "56fdd199312815e2"}, // defaults
	}

	for _, tt := range tests {
		m, _ := ParseGeminiModel(tt.modelName)
		if m != tt.expectedModel {
			t.Errorf("ParseGeminiModel(%q) = %v, want %v", tt.modelName, m, tt.expectedModel)
		}
		if m.Token() != tt.expectedToken {
			t.Errorf("Model.Token() = %q, want %q", m.Token(), tt.expectedToken)
		}
	}

	header := ModelPro.HeaderValue()
	if !strings.Contains(header, `"e6fa609c3fa255c0"`) {
		t.Errorf("header missing model token: %s", header)
	}
}

func TestCustomURLEncode(t *testing.T) {
	input := "hello world * ~ - _ . A"
	expected := "hello+world+*+~+-+_+.+A"
	if got := customURLEncode(input); got != expected {
		t.Errorf("customURLEncode(%q) = %q, want %q", input, got, expected)
	}

	input2 := "hello%world"
	expected2 := "hello%25world"
	if got := customURLEncode(input2); got != expected2 {
		t.Errorf("customURLEncode(%q) = %q, want %q", input2, got, expected2)
	}
}

func TestExtractChunksAndParseEnvelopes(t *testing.T) {
	streamPayload := `)]}'
55
[["wrb.fr","123","[\"hello\", \"world\"]"]]
100
[["wrb.fr","456","[\"ignored\", \"value\"]"], ["wrb.fr","789","[\"choice_data\", 42]"]]`

	envelopes := parseEnvelopes(streamPayload)
	if len(envelopes) != 3 {
		t.Fatalf("expected 3 envelopes, got %d", len(envelopes))
	}

	env1, ok := envelopes[0].([]interface{})
	if !ok || len(env1) != 2 || env1[0].(string) != "hello" {
		t.Errorf("unexpected envelope 1 format: %v", envelopes[0])
	}
}

func TestExtractReply(t *testing.T) {
	innerJSON := `[
		null,
		["conv_123", "resp_456"],
		null,
		null,
		[
			[
				"choice_789",
				["This is the answer text", 0, null, null, null, null, 0],
				null,
				null,
				null,
				null,
				null,
				null,
				null,
				null,
				null,
				null,
				[
					null,
					[
						[
							["https://lh3.googleusercontent.com/path-to-image", 100, 100]
						]
					],
					null,
					null,
					null,
					null,
					null,
					[
						"https://lh3.googleusercontent.com/generated-image.jpg"
					],
					"https://lh3.googleusercontent.com/another-image-sub-tree.mp4"
				]
			]
		]
	]`

	var inner interface{}
	if err := json.Unmarshal([]byte(innerJSON), &inner); err != nil {
		t.Fatalf("failed to setup mock inner json: %v", err)
	}

	reply, ok := extractReply(inner)
	if !ok {
		t.Fatal("failed to extract reply")
	}

	if reply.Text != "This is the answer text" {
		t.Errorf("expected text 'This is the answer text', got %q", reply.Text)
	}
	if reply.ConversationID != "conv_123" {
		t.Errorf("expected conversationID 'conv_123', got %q", reply.ConversationID)
	}
	if reply.ResponseID != "resp_456" {
		t.Errorf("expected responseID 'resp_456', got %q", reply.ResponseID)
	}
	if reply.ChoiceID != "choice_789" {
		t.Errorf("expected choiceID 'choice_789', got %q", reply.ChoiceID)
	}
	if len(reply.WebImageURLs) != 1 || reply.WebImageURLs[0] != "https://lh3.googleusercontent.com/path-to-image" {
		t.Errorf("unexpected WebImageURLs: %v", reply.WebImageURLs)
	}
	if len(reply.GeneratedImageURLs) != 1 || reply.GeneratedImageURLs[0] != "https://lh3.googleusercontent.com/generated-image.jpg" {
		t.Errorf("unexpected GeneratedImageURLs: %v", reply.GeneratedImageURLs)
	}
	if len(reply.GeneratedVideoURLs) != 1 || reply.GeneratedVideoURLs[0] != "https://lh3.googleusercontent.com/another-image-sub-tree.mp4" {
		t.Errorf("unexpected GeneratedVideoURLs: %v", reply.GeneratedVideoURLs)
	}
}

func TestParseStreamResponse(t *testing.T) {
	raw := `)]}'
250
[["wrb.fr","1","[null,[\"c_1\",\"r_1\"],null,null,[[\"ch_1\",[\"Short text\"]]]]"]]
250
[["wrb.fr","2","[null,[\"c_1\",\"r_1\"],null,null,[[\"ch_1\",[\"Much longer response text\"]]]]"]]`

	reply, err := parseStreamResponse(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if reply.Text != "Much longer response text" {
		t.Errorf("expected best text 'Much longer response text', got %q", reply.Text)
	}
}

