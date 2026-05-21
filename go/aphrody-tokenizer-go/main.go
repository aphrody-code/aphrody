// SPDX-License-Identifier: Apache-2.0
package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"sync/atomic"
	"time"

	"github.com/tiktoken-go/tokenizer"
	"golang.org/x/net/html"
	"google.golang.org/genai"
)

// Request defines the JSON schema for stdin requests.
type Request struct {
	Command               string               `json:"command,omitempty"`
	Encoding              string               `json:"encoding,omitempty"`
	Text                  string               `json:"text,omitempty"`
	HTML                  string               `json:"html,omitempty"`
	Model                 string               `json:"model,omitempty"`
	Prompt                string               `json:"prompt,omitempty"`
	SystemInstruction     string               `json:"system_instruction,omitempty"`
	Temperature           *float32             `json:"temperature,omitempty"`
	GoogleSearch          bool                 `json:"google_search,omitempty"`
	ResponseMimeType      string               `json:"response_mime_type,omitempty"`
	ResponseSchema        *genai.Schema        `json:"response_schema,omitempty"`
	FileURI               string               `json:"file_uri,omitempty"`
	FileMIMEType          string               `json:"file_mime_type,omitempty"`
	FilePath              string               `json:"file_path,omitempty"`
	DisplayName           string               `json:"display_name,omitempty"`
	AspectRatio           string               `json:"aspect_ratio,omitempty"`
	NumberOfImages        int32                `json:"number_of_images,omitempty"`
	OutputMIMEType        string               `json:"output_mime_type,omitempty"`
	OutputPath            string               `json:"output_path,omitempty"`
	Contents              []*genai.Content     `json:"contents,omitempty"`
	Tools                 []*genai.Tool        `json:"tools,omitempty"`
	ToolConfig            *genai.ToolConfig    `json:"tool_config,omitempty"`
	CachedContent         string               `json:"cached_content,omitempty"`
	TTLSeconds            int64                `json:"ttl_seconds,omitempty"`
	ExpireTime            string               `json:"expire_time,omitempty"`
	Name                  string               `json:"name,omitempty"`
	BaseModel             string               `json:"base_model,omitempty"`
	TuningDataset         *genai.TuningDataset `json:"tuning_dataset,omitempty"`
	TunedModelDisplayName string               `json:"tuned_model_display_name,omitempty"`
	Description           string               `json:"description,omitempty"`
	EpochCount            *int32               `json:"epoch_count,omitempty"`
	LearningRateMultiplier *float32             `json:"learning_rate_multiplier,omitempty"`
	Filter                string               `json:"filter,omitempty"`
	QueryBase             *bool                `json:"query_base,omitempty"`
	CookiePath            string               `json:"cookie_path,omitempty"`
	CookiesJSON           string               `json:"cookies_json,omitempty"`
	Language              string               `json:"language,omitempty"`
	ConversationId        string               `json:"conversation_id,omitempty"`
	ResponseId            string               `json:"response_id,omitempty"`
	ChoiceId              string               `json:"choice_id,omitempty"`
}

// Response defines the JSON schema for stdout responses.
type Response struct {
	Tokens            int                      `json:"tokens,omitempty"`
	Text              string                   `json:"text,omitempty"`
	Error             string                   `json:"error,omitempty"`
	URI               string                   `json:"uri,omitempty"`
	Name              string                   `json:"name,omitempty"`
	MIMEType          string                   `json:"mime_type,omitempty"`
	GroundingMetadata *genai.GroundingMetadata `json:"grounding_metadata,omitempty"`
	SavedPaths        []string                 `json:"saved_paths,omitempty"`
	TotalTokens       int32                    `json:"total_tokens,omitempty"`
	Embedding         []float32                `json:"embedding,omitempty"`
	Cache             *genai.CachedContent     `json:"cache,omitempty"`
	Caches            []*genai.CachedContent   `json:"caches,omitempty"`
	File              *genai.File              `json:"file,omitempty"`
	Files             []*genai.File            `json:"files,omitempty"`
	ModelInfo         *genai.Model             `json:"model_info,omitempty"`
	ModelsList        []*genai.Model           `json:"models_list,omitempty"`
	TuningJob         *genai.TuningJob         `json:"tuning_job,omitempty"`
	TuningJobsList    []*genai.TuningJob       `json:"tuning_jobs_list,omitempty"`
	WebText           string                   `json:"web_text,omitempty"`
	WebImageURLs      []string                 `json:"web_image_urls,omitempty"`
	GeneratedImageURLs []string                `json:"generated_image_urls,omitempty"`
	GeneratedVideoURLs []string                `json:"generated_video_urls,omitempty"`
	CandidateCount     int                      `json:"candidate_count,omitempty"`
	ConversationId     string                   `json:"conversation_id,omitempty"`
	ResponseId         string                   `json:"response_id,omitempty"`
	ChoiceId           string                   `json:"choice_id,omitempty"`
}

func getEncoding(encName string) (tokenizer.Encoding, error) {
	switch strings.ToLower(encName) {
	case "cl100k_base", "cl100k":
		return tokenizer.Cl100kBase, nil
	case "o200k_base", "o200k":
		return tokenizer.O200kBase, nil
	case "p50k_base", "p50k":
		return tokenizer.P50kBase, nil
	case "r50k_base", "r50k", "gpt2":
		return tokenizer.R50kBase, nil
	default:
		return "", fmt.Errorf("unknown encoding: %s", encName)
	}
}

func isWhitespaceChar(c byte) bool {
	return c == ' ' || c == '\t' || c == '\n' || c == '\r'
}

func endsWithNewline(b []byte) bool {
	if len(b) == 0 {
		return true
	}
	return b[len(b)-1] == '\n'
}

func renderChildrenText(n *html.Node) string {
	var buf bytes.Buffer
	var pendingSpace bool

	var f func(*html.Node)
	f = func(curr *html.Node) {
		if curr.Type == html.ElementNode {
			name := strings.ToLower(curr.Data)
			if name == "script" || name == "style" || name == "head" || name == "noscript" || name == "iframe" || name == "svg" {
				return
			}
			if name == "a" {
				href := ""
				for _, attr := range curr.Attr {
					if strings.ToLower(attr.Key) == "href" {
						href = attr.Val
						break
					}
				}
				linkText := renderChildrenText(curr)
				if linkText != "" {
					if pendingSpace && buf.Len() > 0 {
						buf.WriteByte(' ')
					}
					if href != "" {
						buf.WriteString(fmt.Sprintf("[%s](%s)", linkText, href))
					} else {
						buf.WriteString(linkText)
					}
					pendingSpace = false
				}
				return
			}
			if name == "code" {
				codeText := renderChildrenText(curr)
				if codeText != "" {
					if pendingSpace && buf.Len() > 0 {
						buf.WriteByte(' ')
					}
					buf.WriteString(fmt.Sprintf("`%s`", codeText))
					pendingSpace = false
				}
				return
			}
		}

		if curr.Type == html.TextNode {
			text := curr.Data
			words := strings.Fields(text)
			if len(words) > 0 {
				collapsed := strings.Join(words, " ")
				hasLeading := len(text) > 0 && isWhitespaceChar(text[0])
				hasTrailing := len(text) > 0 && isWhitespaceChar(text[len(text)-1])

				if (hasLeading || pendingSpace) && buf.Len() > 0 {
					buf.WriteByte(' ')
				}
				buf.WriteString(collapsed)
				pendingSpace = hasTrailing
			} else if len(text) > 0 {
				pendingSpace = true
			}
		}

		for c := curr.FirstChild; c != nil; c = c.NextSibling {
			f(c)
		}
	}

	for c := n.FirstChild; c != nil; c = c.NextSibling {
		f(c)
	}
	return strings.TrimSpace(buf.String())
}

// htmlToText converts HTML content into clean markdown-like text.
func htmlToText(htmlInput string) (string, error) {
	doc, err := html.Parse(strings.NewReader(htmlInput))
	if err != nil {
		return "", err
	}

	var buf bytes.Buffer
	var pendingSpace bool

	var f func(*html.Node)
	f = func(n *html.Node) {
		if n.Type == html.ElementNode {
			name := strings.ToLower(n.Data)
			if name == "script" || name == "style" || name == "head" || name == "noscript" || name == "iframe" || name == "svg" {
				return
			}

			switch name {
			case "h1", "h2", "h3", "h4", "h5", "h6":
				buf.WriteString("\n\n")
				level := int(name[1] - '0')
				buf.WriteString(strings.Repeat("#", level) + " ")
				pendingSpace = false
			case "p", "div", "section", "article":
				buf.WriteString("\n\n")
				pendingSpace = false
			case "br":
				buf.WriteString("\n")
				pendingSpace = false
			case "li":
				buf.WriteString("\n- ")
				pendingSpace = false
			case "pre":
				var preText string
				var extractPre func(*html.Node)
				extractPre = func(curr *html.Node) {
					if curr.Type == html.TextNode {
						preText += curr.Data
					}
					for c := curr.FirstChild; c != nil; c = c.NextSibling {
						extractPre(c)
					}
				}
				extractPre(n)
				buf.WriteString("\n```\n" + preText + "\n```\n\n")
				pendingSpace = false
				return
			case "code":
				codeText := renderChildrenText(n)
				if codeText != "" {
					if pendingSpace && buf.Len() > 0 && !endsWithNewline(buf.Bytes()) {
						buf.WriteByte(' ')
					}
					buf.WriteString(fmt.Sprintf("`%s`", codeText))
					pendingSpace = false
				}
				return
			case "tr":
				var cells []string
				for c := n.FirstChild; c != nil; c = c.NextSibling {
					if c.Type == html.ElementNode && (strings.ToLower(c.Data) == "td" || strings.ToLower(c.Data) == "th") {
						cells = append(cells, renderChildrenText(c))
					}
				}
				buf.WriteString("\n| " + strings.Join(cells, " ") + " |")
				pendingSpace = false
				return
			case "a":
				href := ""
				for _, attr := range n.Attr {
					if strings.ToLower(attr.Key) == "href" {
						href = attr.Val
						break
					}
				}
				linkText := renderChildrenText(n)
				if linkText != "" {
					if pendingSpace && buf.Len() > 0 && !endsWithNewline(buf.Bytes()) {
						buf.WriteByte(' ')
					}
					if href != "" {
						buf.WriteString(fmt.Sprintf("[%s](%s)", linkText, href))
					} else {
						buf.WriteString(linkText)
					}
					pendingSpace = false
				}
				return
			}
		}

		if n.Type == html.TextNode {
			text := n.Data
			words := strings.Fields(text)
			if len(words) > 0 {
				collapsed := strings.Join(words, " ")
				hasLeading := len(text) > 0 && isWhitespaceChar(text[0])
				hasTrailing := len(text) > 0 && isWhitespaceChar(text[len(text)-1])

				if (hasLeading || pendingSpace) && buf.Len() > 0 && !endsWithNewline(buf.Bytes()) {
					lastChar := buf.Bytes()[buf.Len()-1]
					if lastChar != ' ' {
						buf.WriteByte(' ')
					}
				}
				buf.WriteString(collapsed)
				pendingSpace = hasTrailing
			} else if len(text) > 0 {
				pendingSpace = true
			}
		}

		for c := n.FirstChild; c != nil; c = c.NextSibling {
			f(c)
		}
	}

	f(doc)

	lines := strings.Split(buf.String(), "\n")
	var resultLines []string
	lastWasEmpty := false

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" {
			if !lastWasEmpty {
				resultLines = append(resultLines, "")
				lastWasEmpty = true
			}
		} else {
			resultLines = append(resultLines, strings.TrimRight(line, " \t"))
			lastWasEmpty = false
		}
	}

	return strings.TrimSpace(strings.Join(resultLines, "\n")), nil
}

func runGemini(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		req.Model = "gemini-2.5-flash"
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	var contents []*genai.Content
	if req.FileURI != "" {
		filePart := genai.NewPartFromURI(req.FileURI, req.FileMIMEType)
		if req.Prompt != "" {
			contents = []*genai.Content{
				{
					Parts: []*genai.Part{
						filePart,
						genai.NewPartFromText(req.Prompt),
					},
					Role: "user",
				},
			}
		} else {
			contents = []*genai.Content{
				{
					Parts: []*genai.Part{filePart},
					Role:  "user",
				},
			}
		}
	} else {
		if req.Prompt == "" {
			return Response{}, fmt.Errorf("prompt is required for gemini command")
		}
		contents = []*genai.Content{
			{
				Parts: []*genai.Part{genai.NewPartFromText(req.Prompt)},
				Role:  "user",
			},
		}
	}

	config := &genai.GenerateContentConfig{}
	if req.SystemInstruction != "" {
		config.SystemInstruction = &genai.Content{
			Parts: []*genai.Part{genai.NewPartFromText(req.SystemInstruction)},
			Role:  "system",
		}
	}
	if req.Temperature != nil {
		config.Temperature = req.Temperature
	}
	if req.GoogleSearch {
		config.Tools = []*genai.Tool{
			{GoogleSearch: &genai.GoogleSearch{}},
		}
	}
	if req.ResponseMimeType != "" {
		config.ResponseMIMEType = req.ResponseMimeType
	}
	if req.ResponseSchema != nil {
		config.ResponseSchema = req.ResponseSchema
	}
	if req.CachedContent != "" {
		config.CachedContent = req.CachedContent
	}

	resp, err := client.Models.GenerateContent(ctx, req.Model, contents, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to generate content: %w", err)
	}

	var grounding *genai.GroundingMetadata
	if len(resp.Candidates) > 0 && resp.Candidates[0].GroundingMetadata != nil {
		grounding = resp.Candidates[0].GroundingMetadata
	}

	return Response{
		Text:              resp.Text(),
		GroundingMetadata: grounding,
	}, nil
}

func runUpload(ctx context.Context, req Request) (Response, error) {
	if req.FilePath == "" {
		return Response{}, fmt.Errorf("file_path is required for upload command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.UploadFileConfig{}
	if req.DisplayName != "" {
		config.DisplayName = req.DisplayName
	}
	if req.FileMIMEType != "" {
		config.MIMEType = req.FileMIMEType
	}

	fileInfo, err := client.Files.UploadFromPath(ctx, req.FilePath, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to upload file: %w", err)
	}

	return Response{
		URI:      fileInfo.URI,
		Name:     fileInfo.Name,
		MIMEType: fileInfo.MIMEType,
	}, nil
}

func runGenerateImage(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		req.Model = "imagen-3.0-generate-002"
	}
	if req.Prompt == "" {
		return Response{}, fmt.Errorf("prompt is required for generate_image command")
	}
	if req.OutputPath == "" {
		return Response{}, fmt.Errorf("output_path is required for generate_image command")
	}

	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.GenerateImagesConfig{}
	if req.AspectRatio != "" {
		config.AspectRatio = req.AspectRatio
	}
	if req.NumberOfImages > 0 {
		config.NumberOfImages = req.NumberOfImages
	} else {
		config.NumberOfImages = 1
	}
	if req.OutputMIMEType != "" {
		config.OutputMIMEType = req.OutputMIMEType
	}

	resp, err := client.Models.GenerateImages(ctx, req.Model, req.Prompt, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to generate images: %w", err)
	}

	if len(resp.GeneratedImages) == 0 {
		return Response{}, fmt.Errorf("no images were generated by the model")
	}

	var savedPaths []string
	for i, genImg := range resp.GeneratedImages {
		if genImg.Image == nil || len(genImg.Image.ImageBytes) == 0 {
			continue
		}
		path := req.OutputPath
		if i > 0 {
			ext := ""
			idx := strings.LastIndex(path, ".")
			if idx != -1 {
				ext = path[idx:]
				path = path[:idx] + fmt.Sprintf("_%d", i) + ext
			} else {
				path = path + fmt.Sprintf("_%d", i)
			}
		}

		err = os.WriteFile(path, genImg.Image.ImageBytes, 0644)
		if err != nil {
			return Response{}, fmt.Errorf("failed to write generated image to %s: %w", path, err)
		}
		savedPaths = append(savedPaths, path)
	}

	if len(savedPaths) == 0 {
		return Response{}, fmt.Errorf("generated images did not contain any valid image bytes")
	}

	return Response{
		SavedPaths: savedPaths,
	}, nil
}

func runCountTokens(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		req.Model = "gemini-2.5-flash"
	}
	if req.Prompt == "" && req.FileURI == "" {
		return Response{}, fmt.Errorf("prompt or file_uri is required for count_tokens command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	var contents []*genai.Content
	if req.FileURI != "" {
		filePart := genai.NewPartFromURI(req.FileURI, req.FileMIMEType)
		if req.Prompt != "" {
			contents = []*genai.Content{
				{
					Parts: []*genai.Part{
						filePart,
						genai.NewPartFromText(req.Prompt),
					},
					Role: "user",
				},
			}
		} else {
			contents = []*genai.Content{
				{
					Parts: []*genai.Part{filePart},
					Role:  "user",
				},
			}
		}
	} else {
		contents = []*genai.Content{
			{
				Parts: []*genai.Part{genai.NewPartFromText(req.Prompt)},
				Role:  "user",
			},
		}
	}

	resp, err := client.Models.CountTokens(ctx, req.Model, contents, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to count tokens: %w", err)
	}

	return Response{
		TotalTokens: resp.TotalTokens,
	}, nil
}

func runEmbedContent(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		req.Model = "text-embedding-004"
	}
	if req.Prompt == "" {
		return Response{}, fmt.Errorf("prompt is required for embed_content command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	contents := []*genai.Content{
		{
			Parts: []*genai.Part{genai.NewPartFromText(req.Prompt)},
			Role:  "user",
		},
	}

	resp, err := client.Models.EmbedContent(ctx, req.Model, contents, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to embed content: %w", err)
	}

	if len(resp.Embeddings) == 0 || resp.Embeddings[0] == nil {
		return Response{}, fmt.Errorf("no embedding returned in response")
	}

	return Response{
		Embedding: resp.Embeddings[0].Values,
	}, nil
}

func main() {
	if len(os.Args) >= 2 {
		if os.Args[1] == "--help" || os.Args[1] == "-h" {
			fmt.Println("Usage:")
			fmt.Println("  aphrody-tokenizer-go count <encoding> <text>                     Print token count as integer")
			fmt.Println("  aphrody-tokenizer-go html2text [text...]                         Convert HTML input to clean text/markdown")
			fmt.Println("  aphrody-tokenizer-go gemini <model> <prompt>                     Call Gemini content generation API")
			fmt.Println("  aphrody-tokenizer-go gemini_web <prompt> [cookie_path]           Call Gemini web client API")
			fmt.Println("  aphrody-tokenizer-go gemini_web_chat [model] [cookie_path]       Interactive console chat over Gemini Web")
			fmt.Println("  aphrody-tokenizer-go upload <file_path> [display_name]           Upload media file via Files API")
			fmt.Println("  aphrody-tokenizer-go generate_image <model> <prompt> <out_path>  Generate image using Imagen")
			fmt.Println("  aphrody-tokenizer-go count_tokens <model> <prompt>               Count tokens using Gemini API")
			fmt.Println("  aphrody-tokenizer-go embed <model> <prompt>                      Generate content embedding using Gemini")
			fmt.Println("  aphrody-tokenizer-go create_cache <model> <prompt> [disp_name]   Create a context cache resource")
			fmt.Println("  aphrody-tokenizer-go get_cache <name>                            Retrieve cache resource metadata")
			fmt.Println("  aphrody-tokenizer-go delete_cache <name>                         Delete a cache resource")
			fmt.Println("  aphrody-tokenizer-go list_caches                                 List all cache resources")
			fmt.Println("  aphrody-tokenizer-go get_file <name>                             Retrieve file resource metadata")
			fmt.Println("  aphrody-tokenizer-go delete_file <name>                          Delete a file resource")
			fmt.Println("  aphrody-tokenizer-go list_files                                  List all uploaded files")
			fmt.Println("  aphrody-tokenizer-go list_models                                 List available models")
			fmt.Println("  aphrody-tokenizer-go get_model <model>                           Retrieve model metadata")
			fmt.Println("  aphrody-tokenizer-go tune_model <base_model> <gcs_uri>           Tune a model (supervised fine-tuning dataset GCS URI)")
			fmt.Println("  aphrody-tokenizer-go get_tuning_job <name>                       Retrieve fine-tuning job metadata")
			fmt.Println("  aphrody-tokenizer-go list_tuning_jobs                            List fine-tuning jobs")
			fmt.Println("  aphrody-tokenizer-go cancel_tuning_job <name>                    Cancel a running fine-tuning job")
			fmt.Println("  aphrody-tokenizer-go live_chat <model> [system_instruction]      Interactive console chat over WebSockets")
			fmt.Println("  aphrody-tokenizer-go                                             Read JSON request from stdin and write JSON to stdout")
			os.Exit(0)
		}

		if os.Args[1] == "html2text" {
			var htmlInput string
			if len(os.Args) >= 3 {
				htmlInput = strings.Join(os.Args[2:], " ")
			} else {
				var buf bytes.Buffer
				_, err := buf.ReadFrom(os.Stdin)
				if err != nil {
					fmt.Fprintf(os.Stderr, "Error reading stdin: %v\n", err)
					os.Exit(1)
				}
				htmlInput = buf.String()
			}
			text, err := htmlToText(htmlInput)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			fmt.Println(text)
			os.Exit(0)
		}

		if len(os.Args) >= 4 && os.Args[1] == "count" {
			encName := os.Args[2]
			text := strings.Join(os.Args[3:], " ")

			enc, err := getEncoding(encName)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}

			codec, err := tokenizer.Get(enc)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error getting codec: %v\n", err)
				os.Exit(1)
			}

			ids, _, err := codec.Encode(text)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error encoding: %v\n", err)
				os.Exit(1)
			}

			fmt.Println(len(ids))
			os.Exit(0)
		}

		if os.Args[1] == "gemini" && len(os.Args) >= 4 {
			ctx := context.Background()
			req := Request{
				Model:  os.Args[2],
				Prompt: strings.Join(os.Args[3:], " "),
			}
			resp, err := runGemini(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "gemini_web" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Prompt: os.Args[2],
			}
			if len(os.Args) >= 4 {
				req.CookiePath = os.Args[3]
			}
			resp, err := runGeminiWeb(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "gemini_web_chat" {
			ctx := context.Background()
			model := ""
			if len(os.Args) >= 3 {
				model = os.Args[2]
			}
			cookiePath := ""
			if len(os.Args) >= 4 {
				cookiePath = os.Args[3]
			}
			err := runGeminiWebChat(ctx, model, cookiePath, "en")
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			os.Exit(0)
		}

		if os.Args[1] == "upload" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				FilePath: os.Args[2],
			}
			if len(os.Args) >= 4 {
				req.DisplayName = os.Args[3]
			}
			resp, err := runUpload(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "generate_image" && len(os.Args) >= 5 {
			ctx := context.Background()
			req := Request{
				Model:      os.Args[2],
				Prompt:     os.Args[3],
				OutputPath: os.Args[4],
			}
			resp, err := runGenerateImage(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "count_tokens" && len(os.Args) >= 4 {
			ctx := context.Background()
			req := Request{
				Model:  os.Args[2],
				Prompt: strings.Join(os.Args[3:], " "),
			}
			resp, err := runCountTokens(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "embed" && len(os.Args) >= 4 {
			ctx := context.Background()
			req := Request{
				Model:  os.Args[2],
				Prompt: strings.Join(os.Args[3:], " "),
			}
			resp, err := runEmbedContent(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "create_cache" && len(os.Args) >= 4 {
			ctx := context.Background()
			req := Request{
				Model:  os.Args[2],
				Prompt: os.Args[3],
			}
			if len(os.Args) >= 5 {
				req.DisplayName = os.Args[4]
			}
			req.Contents = []*genai.Content{
				{
					Parts: []*genai.Part{
						genai.NewPartFromText(req.Prompt),
					},
					Role: "user",
				},
			}
			resp, err := runCreateCache(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "get_cache" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Name: os.Args[2],
			}
			resp, err := runGetCache(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "delete_cache" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Name: os.Args[2],
			}
			resp, err := runDeleteCache(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "list_caches" {
			ctx := context.Background()
			resp, err := runListCaches(ctx, Request{})
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "get_file" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Name: os.Args[2],
			}
			resp, err := runGetFile(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "delete_file" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Name: os.Args[2],
			}
			resp, err := runDeleteFile(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "list_files" {
			ctx := context.Background()
			resp, err := runListFiles(ctx, Request{})
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "list_models" {
			ctx := context.Background()
			resp, err := runListModels(ctx, Request{})
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "get_model" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Model: os.Args[2],
			}
			resp, err := runGetModel(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "tune_model" && len(os.Args) >= 4 {
			ctx := context.Background()
			req := Request{
				BaseModel: os.Args[2],
				TuningDataset: &genai.TuningDataset{
					GCSURI: os.Args[3],
				},
			}
			if len(os.Args) >= 5 {
				req.TunedModelDisplayName = os.Args[4]
			}
			resp, err := runTuneModel(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "get_tuning_job" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Name: os.Args[2],
			}
			resp, err := runGetTuningJob(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "list_tuning_jobs" {
			ctx := context.Background()
			resp, err := runListTuningJobs(ctx, Request{})
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "cancel_tuning_job" && len(os.Args) >= 3 {
			ctx := context.Background()
			req := Request{
				Name: os.Args[2],
			}
			resp, err := runCancelTuningJob(ctx, req)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			_ = json.NewEncoder(os.Stdout).Encode(resp)
			os.Exit(0)
		}

		if os.Args[1] == "live_chat" && len(os.Args) >= 3 {
			ctx := context.Background()
			model := os.Args[2]
			sysInst := ""
			if len(os.Args) >= 4 {
				sysInst = strings.Join(os.Args[3:], " ")
			}
			err := runLiveChat(ctx, model, sysInst)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
			os.Exit(0)
		}
	}

	decoder := json.NewDecoder(os.Stdin)
	var req Request
	if err := decoder.Decode(&req); err != nil {
		writeJSONError("failed to decode JSON input: " + err.Error())
		os.Exit(1)
	}

	if req.Command == "html2text" {
		text, err := htmlToText(req.HTML)
		if err != nil {
			writeJSONError("failed to parse HTML: " + err.Error())
			os.Exit(1)
		}
		resp := Response{
			Text: text,
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	ctx := context.Background()

	if req.Command == "gemini" {
		resp, err := runGemini(ctx, req)
		if err != nil {
			writeJSONError("gemini command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "gemini_web" {
		resp, err := runGeminiWeb(ctx, req)
		if err != nil {
			writeJSONError("gemini_web command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "upload" {
		resp, err := runUpload(ctx, req)
		if err != nil {
			writeJSONError("upload command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "generate_image" {
		resp, err := runGenerateImage(ctx, req)
		if err != nil {
			writeJSONError("generate_image command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "count_tokens" {
		resp, err := runCountTokens(ctx, req)
		if err != nil {
			writeJSONError("count_tokens command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "embed_content" {
		resp, err := runEmbedContent(ctx, req)
		if err != nil {
			writeJSONError("embed_content command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "create_cache" {
		resp, err := runCreateCache(ctx, req)
		if err != nil {
			writeJSONError("create_cache command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "get_cache" {
		resp, err := runGetCache(ctx, req)
		if err != nil {
			writeJSONError("get_cache command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "delete_cache" {
		resp, err := runDeleteCache(ctx, req)
		if err != nil {
			writeJSONError("delete_cache command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "list_caches" {
		resp, err := runListCaches(ctx, req)
		if err != nil {
			writeJSONError("list_caches command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "update_cache" {
		resp, err := runUpdateCache(ctx, req)
		if err != nil {
			writeJSONError("update_cache command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "get_file" {
		resp, err := runGetFile(ctx, req)
		if err != nil {
			writeJSONError("get_file command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "delete_file" {
		resp, err := runDeleteFile(ctx, req)
		if err != nil {
			writeJSONError("delete_file command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "list_files" {
		resp, err := runListFiles(ctx, req)
		if err != nil {
			writeJSONError("list_files command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "list_models" {
		resp, err := runListModels(ctx, req)
		if err != nil {
			writeJSONError("list_models command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "get_model" {
		resp, err := runGetModel(ctx, req)
		if err != nil {
			writeJSONError("get_model command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "tune_model" {
		resp, err := runTuneModel(ctx, req)
		if err != nil {
			writeJSONError("tune_model command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "get_tuning_job" {
		resp, err := runGetTuningJob(ctx, req)
		if err != nil {
			writeJSONError("get_tuning_job command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "list_tuning_jobs" {
		resp, err := runListTuningJobs(ctx, req)
		if err != nil {
			writeJSONError("list_tuning_jobs command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "cancel_tuning_job" {
		resp, err := runCancelTuningJob(ctx, req)
		if err != nil {
			writeJSONError("cancel_tuning_job command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Command == "live_chat" {
		resp, err := runLiveChatIPC(ctx, req)
		if err != nil {
			writeJSONError("live_chat command failed: " + err.Error())
			os.Exit(1)
		}
		_ = json.NewEncoder(os.Stdout).Encode(resp)
		os.Exit(0)
	}

	if req.Encoding == "" {
		req.Encoding = "cl100k_base"
	}

	enc, err := getEncoding(req.Encoding)
	if err != nil {
		writeJSONError(err.Error())
		os.Exit(1)
	}

	codec, err := tokenizer.Get(enc)
	if err != nil {
		writeJSONError("failed to load tokenizer: " + err.Error())
		os.Exit(1)
	}

	ids, _, err := codec.Encode(req.Text)
	if err != nil {
		writeJSONError("failed to encode: " + err.Error())
		os.Exit(1)
	}

	resp := Response{
		Tokens: len(ids),
	}

	encoder := json.NewEncoder(os.Stdout)
	if err := encoder.Encode(resp); err != nil {
		fmt.Fprintf(os.Stderr, "failed to encode response: %v\n", err)
		os.Exit(1)
	}
}

func writeJSONError(errMsg string) {
	resp := Response{
		Error: errMsg,
	}
	_ = json.NewEncoder(os.Stdout).Encode(resp)
}

func makeCreateCachedContentConfig(req Request) (*genai.CreateCachedContentConfig, error) {
	config := &genai.CreateCachedContentConfig{
		DisplayName: req.DisplayName,
	}
	if len(req.Contents) > 0 {
		config.Contents = req.Contents
	}
	if req.SystemInstruction != "" {
		config.SystemInstruction = &genai.Content{
			Parts: []*genai.Part{genai.NewPartFromText(req.SystemInstruction)},
			Role:  "system",
		}
	}
	if len(req.Tools) > 0 {
		config.Tools = req.Tools
	}
	if req.ToolConfig != nil {
		config.ToolConfig = req.ToolConfig
	}
	if req.TTLSeconds > 0 {
		config.TTL = time.Duration(req.TTLSeconds) * time.Second
	}
	if req.ExpireTime != "" {
		t, err := time.Parse(time.RFC3339, req.ExpireTime)
		if err != nil {
			return nil, fmt.Errorf("invalid expire_time (must be RFC3339): %w", err)
		}
		config.ExpireTime = t
	}
	return config, nil
}

func runCreateCache(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		return Response{}, fmt.Errorf("model is required for create_cache command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config, err := makeCreateCachedContentConfig(req)
	if err != nil {
		return Response{}, err
	}

	cache, err := client.Caches.Create(ctx, req.Model, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create cache: %w", err)
	}

	return Response{
		Cache: cache,
	}, nil
}

func runGetCache(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for get_cache command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	cache, err := client.Caches.Get(ctx, req.Name, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to get cache: %w", err)
	}

	return Response{
		Cache: cache,
	}, nil
}

func runDeleteCache(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for delete_cache command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	_, err = client.Caches.Delete(ctx, req.Name, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to delete cache: %w", err)
	}

	return Response{
		Name: req.Name,
	}, nil
}

func runListCaches(ctx context.Context, req Request) (Response, error) {
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	var allCaches []*genai.CachedContent
	page, err := client.Caches.List(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to list caches: %w", err)
	}

	allCaches = append(allCaches, page.Items...)
	for page.NextPageToken != "" {
		page, err = page.Next(ctx)
		if err != nil {
			return Response{}, fmt.Errorf("failed to fetch next caches page: %w", err)
		}
		allCaches = append(allCaches, page.Items...)
	}

	return Response{
		Caches: allCaches,
	}, nil
}

func runUpdateCache(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for update_cache command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.UpdateCachedContentConfig{}
	if req.TTLSeconds > 0 {
		config.TTL = time.Duration(req.TTLSeconds) * time.Second
	}
	if req.ExpireTime != "" {
		t, err := time.Parse(time.RFC3339, req.ExpireTime)
		if err != nil {
			return Response{}, fmt.Errorf("invalid expire_time (must be RFC3339): %w", err)
		}
		config.ExpireTime = t
	}

	cache, err := client.Caches.Update(ctx, req.Name, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to update cache: %w", err)
	}

	return Response{
		Cache: cache,
	}, nil
}

func runGetFile(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for get_file command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	file, err := client.Files.Get(ctx, req.Name, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to get file: %w", err)
	}

	return Response{
		File: file,
	}, nil
}

func runDeleteFile(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for delete_file command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	_, err = client.Files.Delete(ctx, req.Name, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to delete file: %w", err)
	}

	return Response{
		Name: req.Name,
	}, nil
}

func runListFiles(ctx context.Context, req Request) (Response, error) {
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	var allFiles []*genai.File
	page, err := client.Files.List(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to list files: %w", err)
	}

	allFiles = append(allFiles, page.Items...)
	for page.NextPageToken != "" {
		page, err = page.Next(ctx)
		if err != nil {
			return Response{}, fmt.Errorf("failed to fetch next files page: %w", err)
		}
		allFiles = append(allFiles, page.Items...)
	}

	return Response{
		Files: allFiles,
	}, nil
}

func runLiveChat(ctx context.Context, model string, systemInstruction string) error {
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.LiveConnectConfig{
		ResponseModalities: []genai.Modality{genai.ModalityText},
	}
	if systemInstruction != "" {
		config.SystemInstruction = &genai.Content{
			Parts: []*genai.Part{genai.NewPartFromText(systemInstruction)},
			Role:  "system",
		}
	}

	session, err := client.Live.Connect(ctx, model, config)
	if err != nil {
		return fmt.Errorf("failed to connect to Live session: %w", err)
	}
	defer session.Close()

	fmt.Println("Connected to Live session! Type your message and press Enter. Ctrl+C to exit.")

	go func() {
		scanner := bufio.NewScanner(os.Stdin)
		for scanner.Scan() {
			text := scanner.Text()
			if strings.TrimSpace(text) == "" {
				continue
			}
			input := genai.LiveClientContentInput{
				Turns: []*genai.Content{
					{
						Parts: []*genai.Part{
							genai.NewPartFromText(text),
						},
						Role: "user",
					},
				},
			}
			err := session.SendClientContent(input)
			if err != nil {
				fmt.Fprintf(os.Stderr, "\nFailed to send content: %v\n", err)
				return
			}
		}
	}()

	for {
		msg, err := session.Receive()
		if err != nil {
			if strings.Contains(err.Error(), "closed") || strings.Contains(err.Error(), "EOF") {
				break
			}
			return fmt.Errorf("error receiving from session: %w", err)
		}

		if msg.ServerContent != nil && msg.ServerContent.ModelTurn != nil {
			for _, part := range msg.ServerContent.ModelTurn.Parts {
				if part.Text != "" {
					fmt.Print(part.Text)
				}
			}
		}
		if msg.ServerContent != nil && msg.ServerContent.TurnComplete {
			fmt.Println()
		}
	}

	return nil
}

func runLiveChatIPC(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		req.Model = "gemini-2.5-flash"
	}
	if req.Prompt == "" {
		return Response{}, fmt.Errorf("prompt is required for live_chat command")
	}

	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.LiveConnectConfig{
		ResponseModalities: []genai.Modality{genai.ModalityText},
	}
	if req.SystemInstruction != "" {
		config.SystemInstruction = &genai.Content{
			Parts: []*genai.Part{genai.NewPartFromText(req.SystemInstruction)},
			Role:  "system",
		}
	}

	session, err := client.Live.Connect(ctx, req.Model, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to connect to Live session: %w", err)
	}
	defer session.Close()

	input := genai.LiveClientContentInput{
		Turns: []*genai.Content{
			{
				Parts: []*genai.Part{
					genai.NewPartFromText(req.Prompt),
				},
				Role: "user",
			},
		},
	}
	err = session.SendClientContent(input)
	if err != nil {
		return Response{}, fmt.Errorf("failed to send content: %w", err)
	}

	var sb strings.Builder
	for {
		msg, err := session.Receive()
		if err != nil {
			return Response{}, fmt.Errorf("error receiving from session: %w", err)
		}

		if msg.ServerContent != nil && msg.ServerContent.ModelTurn != nil {
			for _, part := range msg.ServerContent.ModelTurn.Parts {
				if part.Text != "" {
					sb.WriteString(part.Text)
				}
			}
		}
		if msg.ServerContent != nil && (msg.ServerContent.TurnComplete || msg.ServerContent.GenerationComplete) {
			break
		}
	}

	return Response{
		Text: sb.String(),
	}, nil
}

func runListModels(ctx context.Context, req Request) (Response, error) {
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.ListModelsConfig{}
	if req.Filter != "" {
		config.Filter = req.Filter
	}
	if req.QueryBase != nil {
		config.QueryBase = req.QueryBase
	}

	var allModels []*genai.Model
	page, err := client.Models.List(ctx, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to list models: %w", err)
	}

	allModels = append(allModels, page.Items...)
	for page.NextPageToken != "" {
		page, err = page.Next(ctx)
		if err != nil {
			return Response{}, fmt.Errorf("failed to fetch next models page: %w", err)
		}
		allModels = append(allModels, page.Items...)
	}

	return Response{
		ModelsList: allModels,
	}, nil
}

func runGetModel(ctx context.Context, req Request) (Response, error) {
	if req.Model == "" {
		return Response{}, fmt.Errorf("model is required for get_model command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	modelInfo, err := client.Models.Get(ctx, req.Model, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to get model info: %w", err)
	}

	return Response{
		ModelInfo: modelInfo,
	}, nil
}

func runTuneModel(ctx context.Context, req Request) (Response, error) {
	if req.BaseModel == "" {
		return Response{}, fmt.Errorf("base_model is required for tune_model command")
	}
	if req.TuningDataset == nil {
		return Response{}, fmt.Errorf("tuning_dataset is required for tune_model command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.CreateTuningJobConfig{}
	if req.TunedModelDisplayName != "" {
		config.TunedModelDisplayName = req.TunedModelDisplayName
	}
	if req.Description != "" {
		config.Description = req.Description
	}
	if req.EpochCount != nil {
		config.EpochCount = req.EpochCount
	}
	if req.LearningRateMultiplier != nil {
		config.LearningRateMultiplier = req.LearningRateMultiplier
	}

	tuningJob, err := client.Tunings.Tune(ctx, req.BaseModel, req.TuningDataset, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create tuning job: %w", err)
	}

	return Response{
		TuningJob: tuningJob,
	}, nil
}

func runGetTuningJob(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for get_tuning_job command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	tuningJob, err := client.Tunings.Get(ctx, req.Name, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to get tuning job: %w", err)
	}

	return Response{
		TuningJob: tuningJob,
	}, nil
}

func runListTuningJobs(ctx context.Context, req Request) (Response, error) {
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	config := &genai.ListTuningJobsConfig{}
	if req.Filter != "" {
		config.Filter = req.Filter
	}

	var allJobs []*genai.TuningJob
	page, err := client.Tunings.List(ctx, config)
	if err != nil {
		return Response{}, fmt.Errorf("failed to list tuning jobs: %w", err)
	}

	allJobs = append(allJobs, page.Items...)
	for page.NextPageToken != "" {
		page, err = page.Next(ctx)
		if err != nil {
			return Response{}, fmt.Errorf("failed to fetch next tuning jobs page: %w", err)
		}
		allJobs = append(allJobs, page.Items...)
	}

	return Response{
		TuningJobsList: allJobs,
	}, nil
}

func runCancelTuningJob(ctx context.Context, req Request) (Response, error) {
	if req.Name == "" {
		return Response{}, fmt.Errorf("name is required for cancel_tuning_job command")
	}
	client, err := genai.NewClient(ctx, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to create GenAI client: %w", err)
	}

	_, err = client.Tunings.Cancel(ctx, req.Name, nil)
	if err != nil {
		return Response{}, fmt.Errorf("failed to cancel tuning job: %w", err)
	}

	return Response{
		Name: req.Name,
	}, nil
}

// SessionCookie represents one entry in the cookie jar.
type SessionCookie struct {
	Name     string `json:"name"`
	Value    string `json:"value"`
	Domain   string `json:"domain,omitempty"`
	Path     string `json:"path,omitempty"`
	Secure   bool   `json:"secure,omitempty"`
	HttpOnly bool   `json:"httpOnly,omitempty"`
}

// UnmarshalJSON implements custom unmarshaling to support both camelCase and snake_case spellings.
func (s *SessionCookie) UnmarshalJSON(data []byte) error {
	type Alias SessionCookie
	aux := &struct {
		HttpOnlyCamel *bool `json:"httpOnly"`
		HttpOnlySnake *bool `json:"http_only"`
		*Alias
	}{
		Alias: (*Alias)(s),
	}
	if err := json.Unmarshal(data, &aux); err != nil {
		return err
	}
	if aux.HttpOnlyCamel != nil {
		s.HttpOnly = *aux.HttpOnlyCamel
	} else if aux.HttpOnlySnake != nil {
		s.HttpOnly = *aux.HttpOnlySnake
	}
	if s.Path == "" {
		s.Path = "/"
	}
	return nil
}

// CookieJar collection of cookies.
type CookieJar struct {
	Cookies map[string]SessionCookie
}

// NewCookieJar builds a new jar.
func NewCookieJar() *CookieJar {
	return &CookieJar{
		Cookies: make(map[string]SessionCookie),
	}
}

// Insert inserts a cookie into the jar.
func (j *CookieJar) Insert(c SessionCookie) {
	j.Cookies[c.Name] = c
}

// HeaderValue builds the semicolon-separated cookie header value.
func (j *CookieJar) HeaderValue() string {
	var parts []string
	var keys []string
	for k := range j.Cookies {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		c := j.Cookies[k]
		parts = append(parts, fmt.Sprintf("%s=%s", c.Name, c.Value))
	}
	return strings.Join(parts, "; ")
}

// RequireGoogleSession checks for necessary session cookies.
func (j *CookieJar) RequireGoogleSession() error {
	for _, needed := range []string{"SAPISID", "__Secure-1PSID"} {
		if _, ok := j.Cookies[needed]; !ok {
			return fmt.Errorf("cookie jar is missing `%s` — re-export the Google session", needed)
		}
	}
	return nil
}

// Auth credentials carrier.
type Auth struct {
	Jar *CookieJar
}

// NewAuthFromCookieEditorJSON parses cookies from JSON.
func NewAuthFromCookieEditorJSON(payload string) (*Auth, error) {
	var raw []SessionCookie
	if err := json.Unmarshal([]byte(payload), &raw); err != nil {
		return nil, fmt.Errorf("malformed cookie JSON: %w", err)
	}
	jar := NewCookieJar()
	for _, cookie := range raw {
		jar.Insert(cookie)
	}
	if err := jar.RequireGoogleSession(); err != nil {
		return nil, err
	}
	return &Auth{Jar: jar}, nil
}

// NewAuthFromCookieFile loads cookies from a file.
func NewAuthFromCookieFile(path string) (*Auth, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	return NewAuthFromCookieEditorJSON(string(data))
}

// defaultCookiePath returns the default cookie jar file path.
func defaultCookiePath() (string, error) {
	home := os.Getenv("HOME")
	if home == "" {
		home = os.Getenv("USERPROFILE")
	}
	if home == "" {
		return "", fmt.Errorf("cannot resolve HOME/USERPROFILE for default cookie path")
	}
	return filepath.Join(home, ".aphrody", "google-cookies.json"), nil
}

var (
	reAt   = regexp.MustCompile(`"SNlM0e":"([^"]+)"`)
	reBl   = regexp.MustCompile(`"cfb2h":"([^"]+)"`)
	reFsid = regexp.MustCompile(`"FdrFJe":"([^"]+)"`)
)

// SessionTokens represents the CSRF and session identifiers.
type SessionTokens struct {
	At       string `json:"at"`
	Bl       string `json:"bl"`
	Fsid     string `json:"fsid,omitempty"`
	Language string `json:"language,omitempty"`
}

// parseTokensFromHTML parses tokens from raw bootstrap HTML content.
func parseTokensFromHTML(htmlContent string, language string) (SessionTokens, error) {
	atMatches := reAt.FindStringSubmatch(htmlContent)
	if len(atMatches) < 2 {
		return SessionTokens{}, fmt.Errorf("SNlM0e (at) token not found in page — not signed in?")
	}
	blMatches := reBl.FindStringSubmatch(htmlContent)
	if len(blMatches) < 2 {
		return SessionTokens{}, fmt.Errorf("cfb2h (bl) token not found in page")
	}
	var fsid string
	fsidMatches := reFsid.FindStringSubmatch(htmlContent)
	if len(fsidMatches) >= 2 {
		fsid = fsidMatches[1]
	}
	return SessionTokens{
		At:       atMatches[1],
		Bl:       blMatches[1],
		Fsid:     fsid,
		Language: language,
	}, nil
}

const userAgentChrome = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
const urlApp = "https://gemini.google.com/app"

// fetchSessionTokens scrapes tokens from gemini.google.com/app.
func fetchSessionTokens(ctx context.Context, auth *Auth, language string) (SessionTokens, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", urlApp, nil)
	if err != nil {
		return SessionTokens{}, fmt.Errorf("failed to create bootstrap request: %w", err)
	}
	req.Header.Set("User-Agent", userAgentChrome)
	req.Header.Set("Cookie", auth.Jar.HeaderValue())

	client := &http.Client{
		Timeout: 30 * time.Second,
	}
	resp, err := client.Do(req)
	if err != nil {
		return SessionTokens{}, fmt.Errorf("failed to fetch bootstrap page: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusUnauthorized || resp.StatusCode == http.StatusForbidden {
		return SessionTokens{}, fmt.Errorf("%d: Gemini app page rejected the cookie jar (signed out?)", resp.StatusCode)
	}
	if resp.StatusCode != http.StatusOK {
		return SessionTokens{}, fmt.Errorf("unexpected status %d fetching bootstrap page", resp.StatusCode)
	}

	bodyBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return SessionTokens{}, fmt.Errorf("failed to read bootstrap response: %w", err)
	}

	return parseTokensFromHTML(string(bodyBytes), language)
}

// GeminiModel represents a web picker model.
type GeminiModel int

const (
	ModelFlashLite GeminiModel = iota
	ModelFlash
	ModelPro
)

// Token returns model-picker hex token.
func (m GeminiModel) Token() string {
	switch m {
	case ModelFlashLite:
		return "1d44b34bcaa1c04d"
	case ModelFlash:
		return "56fdd199312815e2"
	case ModelPro:
		return "e6fa609c3fa255c0"
	default:
		return ""
	}
}

// VariantIndex returns variant index.
func (m GeminiModel) VariantIndex() int {
	switch m {
	case ModelFlashLite:
		return 6
	case ModelFlash:
		return 1
	case ModelPro:
		return 3
	default:
		return 0
	}
}

// ID returns lowercase string id.
func (m GeminiModel) ID() string {
	switch m {
	case ModelFlashLite:
		return "flash-lite"
	case ModelFlash:
		return "flash"
	case ModelPro:
		return "pro"
	default:
		return ""
	}
}

// ParseGeminiModel parses model by ID.
func ParseGeminiModel(s string) (GeminiModel, bool) {
	switch strings.ToLower(strings.TrimSpace(s)) {
	case "flash-lite", "flashlite", "flash_lite", "3.1-flash-lite":
		return ModelFlashLite, true
	case "flash", "3.5-flash", "flash-3.5", "":
		return ModelFlash, true
	case "pro", "3.1-pro", "pro-3.1":
		return ModelPro, true
	default:
		return ModelFlash, false
	}
}

const defaultClientUUID = "00000000-0000-4000-8000-000000000001"

// HeaderValue formats the JSPB header with default UUID.
func (m GeminiModel) HeaderValue() string {
	return m.HeaderValueWithUUID(defaultClientUUID)
}

// HeaderValueWithUUID formats the JSPB header with custom UUID.
func (m GeminiModel) HeaderValueWithUUID(uuid string) string {
	arr := []interface{}{
		1, nil, nil, nil,
		m.Token(),
		nil, nil, 0,
		[]int{4, 5, 6, 8},
		nil, nil, 3, nil, nil,
		m.VariantIndex(), 1,
		uuid,
	}
	bytes, _ := json.Marshal(arr)
	return string(bytes)
}

// buildSendPayload constructs inner f.req generate structure.
func buildSendPayload(prompt string, language string, conversationID, responseID, choiceID string) ([]interface{}, error) {
	messageContent := []interface{}{prompt, 0, nil, nil, nil, nil, 0}
	languageTuple := []interface{}{language}
	
	var cID interface{} = nil
	if conversationID != "" {
		cID = conversationID
	}
	var rID interface{} = nil
	if responseID != "" {
		rID = responseID
	}
	var chID interface{} = nil
	if choiceID != "" {
		chID = choiceID
	}
	
	chatMetadata := []interface{}{cID, rID, chID}
	
	return []interface{}{messageContent, languageTuple, chatMetadata}, nil
}

// customURLEncode escapes characters in RFC 3986 style matching Rust.
func customURLEncode(input string) string {
	var sb strings.Builder
	hexChars := "0123456789ABCDEF"
	for i := 0; i < len(input); i++ {
		b := input[i]
		switch {
		case (b >= 'A' && b <= 'Z') || (b >= 'a' && b <= 'z') || (b >= '0' && b <= '9') || b == '-' || b == '_' || b == '.' || b == '~' || b == '*':
			sb.WriteByte(b)
		case b == ' ':
			sb.WriteByte('+')
		default:
			sb.WriteByte('%')
			sb.WriteByte(hexChars[b>>4])
			sb.WriteByte(hexChars[b&0x0f])
		}
	}
	return sb.String()
}

// stripXssi removes the safety header prefix.
func stripXssi(raw string) string {
	stripped := raw
	if strings.HasPrefix(stripped, ")]}'\n") {
		stripped = stripped[5:]
	} else if strings.HasPrefix(stripped, ")]}'") {
		stripped = stripped[4:]
	}
	return strings.TrimSpace(stripped)
}

// extractChunks splits length-prefixed stream segments.
func extractChunks(body string) []interface{} {
	var chunks []interface{}
	lines := strings.Split(body, "\n")
	i := 0
	for i < len(lines) {
		line := strings.TrimSpace(lines[i])
		if line == "" {
			i++
			continue
		}
		
		isDigitOnly := true
		if len(line) == 0 {
			isDigitOnly = false
		}
		for _, c := range line {
			if c < '0' || c > '9' {
				isDigitOnly = false
				break
			}
		}
		
		if isDigitOnly {
			length, _ := strconv.Atoi(line)
			if i+1 < len(lines) {
				next := strings.TrimSpace(lines[i+1])
				if next != "" {
					var val interface{}
					if err := json.Unmarshal([]byte(next), &val); err == nil {
						chunks = append(chunks, val)
						i += 2
						continue
					}
				}
			}
			
			var accumulated strings.Builder
			j := i + 1
			for j < len(lines) && accumulated.Len() < length {
				if accumulated.Len() > 0 {
					accumulated.WriteByte('\n')
				}
				accumulated.WriteString(lines[j])
				j++
			}
			var val interface{}
			if err := json.Unmarshal([]byte(strings.TrimSpace(accumulated.String())), &val); err == nil {
				chunks = append(chunks, val)
			}
			i = j
		} else {
			var val interface{}
			if err := json.Unmarshal([]byte(line), &val); err == nil {
				chunks = append(chunks, val)
				i++
			} else {
				i++
			}
		}
	}
	return chunks
}

// parseEnvelopes decodes envelopes from chunk list.
func parseEnvelopes(raw string) []interface{} {
	stripped := stripXssi(raw)
	chunks := extractChunks(stripped)
	var results []interface{}
	
	for _, chunk := range chunks {
		arr, ok := chunk.([]interface{})
		if !ok {
			continue
		}
		for _, env := range arr {
			items, ok := env.([]interface{})
			if !ok {
				continue
			}
			if len(items) == 0 {
				continue
			}
			firstStr, ok := items[0].(string)
			if !ok || firstStr != "wrb.fr" {
				continue
			}
			if len(items) > 2 {
				innerStr, ok := items[2].(string)
				if !ok {
					continue
				}
				var parsed interface{}
				if err := json.Unmarshal([]byte(innerStr), &parsed); err == nil {
					results = append(results, parsed)
				}
			}
		}
	}
	return results
}

// collectImageURLs extracts images recursively from a candidate subtree.
func collectImageURLs(v interface{}) []string {
	var out []string
	var walk func(interface{})
	walk = func(node interface{}) {
		switch val := node.(type) {
		case string:
			isImage := (strings.HasPrefix(val, "https://lh3.googleusercontent.com") ||
				strings.HasPrefix(val, "https://www.gstatic.com") ||
				strings.Contains(val, "googleusercontent.com/")) &&
				!strings.Contains(val, ".mp4")
			if isImage {
				found := false
				for _, u := range out {
					if u == val {
						found = true
						break
					}
				}
				if !found {
					out = append(out, val)
				}
			}
		case []interface{}:
			for _, item := range val {
				walk(item)
			}
		case map[string]interface{}:
			for _, item := range val {
				walk(item)
			}
		}
	}
	walk(v)
	return out
}

// collectVideoURLs extracts video URLs recursively from a candidate subtree.
func collectVideoURLs(v interface{}) []string {
	var out []string
	var walk func(interface{})
	walk = func(node interface{}) {
		switch val := node.(type) {
		case string:
			isVideo := strings.HasPrefix(val, "https://") &&
				(strings.Contains(val, ".mp4") ||
					strings.Contains(val, "/video/") ||
					strings.Contains(val, "videoplayback") ||
					(strings.Contains(val, "googleusercontent.com") && strings.Contains(val, "video")))
			if isVideo {
				found := false
				for _, u := range out {
					if u == val {
						found = true
						break
					}
				}
				if !found {
					out = append(out, val)
				}
			}
		case []interface{}:
			for _, item := range val {
				walk(item)
			}
		case map[string]interface{}:
			for _, item := range val {
				walk(item)
			}
		}
	}
	walk(v)
	return out
}

// ChatReply holds one response iteration.
type ChatReply struct {
	Text               string   `json:"text"`
	ConversationID     string   `json:"conversation_id,omitempty"`
	ResponseID         string   `json:"response_id,omitempty"`
	ChoiceID           string   `json:"choice_id,omitempty"`
	WebImageURLs       []string `json:"web_image_urls,omitempty"`
	GeneratedImageURLs []string `json:"generated_image_urls,omitempty"`
	GeneratedVideoURLs []string `json:"generated_video_urls,omitempty"`
	CandidateCount     int      `json:"candidate_count"`
}

// extractReply decodes fields from envelope payload.
func extractReply(inner interface{}) (*ChatReply, bool) {
	innerArr, ok := inner.([]interface{})
	if !ok || len(innerArr) <= 4 {
		return nil, false
	}
	
	candidates, ok := innerArr[4].([]interface{})
	if !ok || len(candidates) == 0 {
		return nil, false
	}
	candidateCount := len(candidates)
	
	firstCandidate, ok := candidates[0].([]interface{})
	if !ok || len(firstCandidate) <= 1 {
		return nil, false
	}
	
	textArr, ok := firstCandidate[1].([]interface{})
	if !ok || len(textArr) == 0 {
		return nil, false
	}
	
	text, ok := textArr[0].(string)
	if !ok {
		return nil, false
	}
	
	var conversationID, responseID, choiceID string
	
	metaArr, ok := innerArr[1].([]interface{})
	if ok && len(metaArr) > 0 {
		if cid, ok := metaArr[0].(string); ok {
			conversationID = cid
		}
		if len(metaArr) > 1 {
			if rid, ok := metaArr[1].(string); ok {
				responseID = rid
			}
		}
	}
	
	if chid, ok := firstCandidate[0].(string); ok {
		choiceID = chid
	}
	
	var webImageURLs []string
	if len(firstCandidate) > 12 {
		if cand12, ok := firstCandidate[12].([]interface{}); ok {
			if len(cand12) > 1 {
				if imgs, ok := cand12[1].([]interface{}); ok {
					for _, img := range imgs {
						if imgArr, ok := img.([]interface{}); ok && len(imgArr) > 0 {
							if imgUrlArr, ok := imgArr[0].([]interface{}); ok && len(imgUrlArr) > 0 {
								if u, ok := imgUrlArr[0].(string); ok {
									webImageURLs = append(webImageURLs, u)
								}
							}
						}
					}
				}
			}
		}
	}
	
	var generatedImageURLs []string
	var generatedVideoURLs []string
	if len(firstCandidate) > 12 {
		cand12 := firstCandidate[12]
		if cand12Arr, ok := cand12.([]interface{}); ok {
			if len(cand12Arr) > 7 {
				generatedImageURLs = collectImageURLs(cand12Arr[7])
			}
			generatedVideoURLs = collectVideoURLs(cand12)
		}
	}
	
	return &ChatReply{
		Text:               text,
		ConversationID:     conversationID,
		ResponseID:         responseID,
		ChoiceID:           choiceID,
		WebImageURLs:       webImageURLs,
		GeneratedImageURLs: generatedImageURLs,
		GeneratedVideoURLs: generatedVideoURLs,
		CandidateCount:     candidateCount,
	}, true
}

// parseStreamResponse extracts best candidate from streaming envelopes.
func parseStreamResponse(raw string) (*ChatReply, error) {
	var best *ChatReply
	envelopes := parseEnvelopes(raw)
	for _, inner := range envelopes {
		if reply, ok := extractReply(inner); ok {
			if best == nil || len(reply.Text) > len(best.Text) {
				best = reply
			}
		}
	}
	if best == nil {
		return nil, fmt.Errorf("StreamGenerate: no reply candidate in stream")
	}
	return best, nil
}

// HttpTransport wraps client + config.
type HttpTransport struct {
	Client     *http.Client
	Auth       *Auth
	Tokens     SessionTokens
	ReqCounter int64
}

// NewHttpTransport builds transport.
func NewHttpTransport(auth *Auth, tokens SessionTokens) *HttpTransport {
	return &HttpTransport{
		Client: &http.Client{
			Timeout: 2 * time.Minute,
		},
		Auth:       auth,
		Tokens:     tokens,
		ReqCounter: 100000,
	}
}

// NextReqID yields next identifier.
func (t *HttpTransport) NextReqID() int64 {
	return atomic.AddInt64(&t.ReqCounter, 100000)
}

const urlStreamGenerate = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
const modelHeader = "x-goog-ext-525001261-jspb"

// StreamGenerate executes network request.
func (t *HttpTransport) StreamGenerate(ctx context.Context, inner interface{}, model string) (string, error) {
	innerBytes, err := json.Marshal(inner)
	if err != nil {
		return "", fmt.Errorf("failed to marshal inner payload: %w", err)
	}
	
	envelope := []interface{}{nil, string(innerBytes)}
	envelopeBytes, err := json.Marshal(envelope)
	if err != nil {
		return "", fmt.Errorf("failed to marshal envelope: %w", err)
	}
	
	body := fmt.Sprintf("f.req=%s&at=%s", customURLEncode(string(envelopeBytes)), customURLEncode(t.Tokens.At))
	
	reqID := t.NextReqID()
	
	parsedURL, err := url.Parse(urlStreamGenerate)
	if err != nil {
		return "", fmt.Errorf("invalid stream URL: %w", err)
	}
	
	q := parsedURL.Query()
	q.Set("bl", t.Tokens.Bl)
	lang := t.Tokens.Language
	if lang == "" {
		lang = "en"
	}
	q.Set("hl", lang)
	q.Set("_reqid", strconv.FormatInt(reqID, 10))
	q.Set("rt", "c")
	if t.Tokens.Fsid != "" {
		q.Set("f.sid", t.Tokens.Fsid)
	}
	parsedURL.RawQuery = q.Encode()
	
	req, err := http.NewRequestWithContext(ctx, "POST", parsedURL.String(), strings.NewReader(body))
	if err != nil {
		return "", fmt.Errorf("failed to create request: %w", err)
	}
	
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
	req.Header.Set("Content-Length", strconv.Itoa(len(body)))
	req.Header.Set("Origin", "https://gemini.google.com")
	req.Header.Set("Referer", "https://gemini.google.com/")
	req.Header.Set("x-same-domain", "1")
	req.Header.Set("User-Agent", userAgentChrome)
	req.Header.Set("Cookie", t.Auth.Jar.HeaderValue())
	if model != "" {
		req.Header.Set(modelHeader, model)
	}
	
	resp, err := t.Client.Do(req)
	if err != nil {
		return "", fmt.Errorf("POST request failed: %w", err)
	}
	defer resp.Body.Close()
	
	if resp.StatusCode == http.StatusUnauthorized || resp.StatusCode == http.StatusForbidden {
		return "", fmt.Errorf("%d: StreamGenerate rejected", resp.StatusCode)
	}
	
	respBytes, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read stream response: %w", err)
	}
	
	if resp.StatusCode != http.StatusOK {
		msg := string(respBytes)
		if len(msg) > 200 {
			msg = msg[:200]
		}
		return "", fmt.Errorf("StreamGenerate RPC error: status %d, message: %s", resp.StatusCode, msg)
	}
	
	return string(respBytes), nil
}

// GeminiWebClient orchestration façade.
type GeminiWebClient struct {
	Transport *HttpTransport
	Language  string
}

// NewGeminiWebClient bootstraps client.
func NewGeminiWebClient(auth *Auth, language string) (*GeminiWebClient, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	
	tokens, err := fetchSessionTokens(ctx, auth, language)
	if err != nil {
		return nil, err
	}
	
	transport := NewHttpTransport(auth, tokens)
	return &GeminiWebClient{
		Transport: transport,
		Language:  language,
	}, nil
}

// Send executes model prompts.
func (c *GeminiWebClient) Send(ctx context.Context, prompt string, model string, conversationID, responseID, choiceID string) (*ChatReply, error) {
	payload, err := buildSendPayload(prompt, c.Language, conversationID, responseID, choiceID)
	if err != nil {
		return nil, err
	}
	
	raw, err := c.Transport.StreamGenerate(ctx, payload, model)
	if err != nil {
		return nil, err
	}
	
	return parseStreamResponse(raw)
}

// runGeminiWeb processes the JSON request or one-shot command.
func runGeminiWeb(ctx context.Context, req Request) (Response, error) {
	if req.Prompt == "" {
		return Response{}, fmt.Errorf("prompt is required for gemini_web command")
	}
	
	var auth *Auth
	var err error
	if req.CookiesJSON != "" {
		auth, err = NewAuthFromCookieEditorJSON(req.CookiesJSON)
	} else {
		path := req.CookiePath
		if path == "" {
			path, err = defaultCookiePath()
			if err != nil {
				return Response{}, err
			}
		}
		auth, err = NewAuthFromCookieFile(path)
	}
	if err != nil {
		return Response{}, fmt.Errorf("auth initialization failed: %w", err)
	}
	
	lang := req.Language
	if lang == "" {
		lang = "en"
	}
	
	client, err := NewGeminiWebClient(auth, lang)
	if err != nil {
		return Response{}, fmt.Errorf("client creation failed: %w", err)
	}
	
	modelHeaderVal := ""
	if req.Model != "" {
		m, ok := ParseGeminiModel(req.Model)
		if ok {
			modelHeaderVal = m.HeaderValue()
		} else {
			if strings.HasPrefix(req.Model, "[") {
				modelHeaderVal = req.Model
			}
		}
	}
	
	reply, err := client.Send(ctx, req.Prompt, modelHeaderVal, req.ConversationId, req.ResponseId, req.ChoiceId)
	if err != nil {
		return Response{}, fmt.Errorf("send failed: %w", err)
	}
	
	return Response{
		WebText:            reply.Text,
		WebImageURLs:       reply.WebImageURLs,
		GeneratedImageURLs: reply.GeneratedImageURLs,
		GeneratedVideoURLs: reply.GeneratedVideoURLs,
		CandidateCount:     reply.CandidateCount,
		ConversationId:     reply.ConversationID,
		ResponseId:         reply.ResponseID,
		ChoiceId:           reply.ChoiceID,
	}, nil
}

// runGeminiWebChat runs interactive console.
func runGeminiWebChat(ctx context.Context, model string, cookiePath string, language string) error {
	var auth *Auth
	var err error
	if cookiePath == "" {
		cookiePath, err = defaultCookiePath()
		if err != nil {
			return err
		}
	}
	auth, err = NewAuthFromCookieFile(cookiePath)
	if err != nil {
		return fmt.Errorf("failed to load cookie file: %w", err)
	}
	
	if language == "" {
		language = "en"
	}
	
	client, err := NewGeminiWebClient(auth, language)
	if err != nil {
		return fmt.Errorf("failed to initialize Gemini web client: %w", err)
	}
	
	modelHeaderVal := ""
	if model != "" {
		m, ok := ParseGeminiModel(model)
		if ok {
			modelHeaderVal = m.HeaderValue()
		}
	}
	
	fmt.Printf("Connected to Gemini Web (%s)! Type your message and press Enter. Ctrl+C to exit.\n\n", language)
	
	var conversationID, responseID, choiceID string
	scanner := bufio.NewScanner(os.Stdin)
	for {
		fmt.Print("You> ")
		if !scanner.Scan() {
			break
		}
		text := scanner.Text()
		if strings.TrimSpace(text) == "" {
			continue
		}
		
		fmt.Print("Gemini> ")
		reply, err := client.Send(ctx, text, modelHeaderVal, conversationID, responseID, choiceID)
		if err != nil {
			fmt.Printf("\nError: %v\n\n", err)
			continue
		}
		
		fmt.Println(reply.Text)
		if len(reply.GeneratedImageURLs) > 0 {
			fmt.Println("Generated images:")
			for _, img := range reply.GeneratedImageURLs {
				fmt.Printf("  - %s\n", img)
			}
		}
		if len(reply.GeneratedVideoURLs) > 0 {
			fmt.Println("Generated videos:")
			for _, vid := range reply.GeneratedVideoURLs {
				fmt.Printf("  - %s\n", vid)
			}
		}
		fmt.Println()
		
		conversationID = reply.ConversationID
		responseID = reply.ResponseID
		choiceID = reply.ChoiceID
	}
	return nil
}

