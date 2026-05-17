// SPDX-License-Identifier: Apache-2.0
import type { JSX, ReactNode } from "react";
import "./globals.css";

// `Metadata` / `Viewport` are typed in `next/dist/types` which only exists
// once the fork has been built. We declare a structural local type so this
// file typechecks against either a built or unbuilt fork.
interface Metadata {
  title?: string;
  description?: string;
  applicationName?: string;
}

interface Viewport {
  width?: string;
  initialScale?: number;
  themeColor?: string;
}

export const metadata: Metadata = {
  title: "Gemini — aphrody",
  description:
    "Pixel-perfect aphrody fork of gemini.google.com/app — M3 tokens + Google Sans Flex + WebGPU spectrum-shift, voice-first.",
  applicationName: "aphrody/gemini-app-aphrody",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#FEF7FF",
};

export default function RootLayout({
  children,
}: {
  children: ReactNode;
}): JSX.Element {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
