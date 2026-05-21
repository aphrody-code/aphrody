// SPDX-License-Identifier: Apache-2.0
package main

import (
	"testing"

	"github.com/tiktoken-go/tokenizer"
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
