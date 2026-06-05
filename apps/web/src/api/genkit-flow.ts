// SPDX-License-Identifier: Apache-2.0

import { genkit, z } from "genkit";
import { googleAI } from "@genkit-ai/google-genai";

const apiKey = process.env.GEMINI_API_KEY || process.env.GOOGLE_API_KEY;

const ai = genkit({
  plugins: [
    googleAI({
      apiKey: apiKey,
    }),
  ],
});

export const aphrodyFlow = ai.defineFlow(
  {
    name: "aphrodyFlow",
    inputSchema: z.object({ prompt: z.string() }),
    outputSchema: z.object({ response: z.string() }),
  },
  async ({ prompt }) => {
    const response = await ai.generate({
      model: googleAI.model("gemini-2.5-flash"),
      prompt: prompt,
    });
    return { response: response.text || "" };
  }
);
