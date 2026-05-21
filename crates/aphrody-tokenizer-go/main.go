// SPDX-License-Identifier: Apache-2.0
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/tiktoken-go/tokenizer"
	"golang.org/x/net/html"
)

// Request defines the JSON schema for stdin requests.
type Request struct {
	Command  string `json:"command,omitempty"`
	Encoding string `json:"encoding,omitempty"`
	Text     string `json:"text,omitempty"`
	HTML     string `json:"html,omitempty"`
}

// Response defines the JSON schema for stdout responses.
type Response struct {
	Tokens int    `json:"tokens,omitempty"`
	Text   string `json:"text,omitempty"`
	Error  string `json:"error,omitempty"`
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

func main() {
	if len(os.Args) >= 2 {
		if os.Args[1] == "--help" || os.Args[1] == "-h" {
			fmt.Println("Usage:")
			fmt.Println("  aphrody-tokenizer-go count <encoding> <text>   Print token count as integer")
			fmt.Println("  aphrody-tokenizer-go html2text [text...]       Convert HTML input to clean text/markdown")
			fmt.Println("  aphrody-tokenizer-go                           Read JSON request from stdin and write JSON to stdout")
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
