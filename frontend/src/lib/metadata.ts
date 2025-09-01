import type { Metadata } from "next"

export const siteTitle = "inkathon"
export const siteDescription =
  "Next generation full-stack boilerplate for ink! smart contracts running on PolkaVM. Powered by Papi, ReactiveDOT, Pop CLI, and more."

export const siteBanner = {
  url: "/inkathon-og-banner.jpg",
  width: 1200,
  height: 630,
  alt: "inkathon Boilerplate",
}

export const siteMetadata: Metadata = {
  title: siteTitle,
  description: siteDescription,
  metadataBase: new URL("https://ink-merkle-aridrop.vercel.app"),
  openGraph: {
    images: [siteBanner],
  },
  twitter: {
    images: [siteBanner],
    description: siteDescription,
    card: "summary_large_image",
    site: "@ink-merkle-aridrop.vercel.app",
    creator: "@truthixify",
  },
  authors: [{ name: "truthixify", url: "https://truthixify.vercel.app" }],
}
