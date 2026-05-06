import type { Metadata, Viewport } from "next";
import type { PropsWithChildren } from "react";

export const metadata: Metadata = {
	title: "Design — Cap",
	description:
		"The Cap design system. Tokens, typography, components and patterns.",
	robots: { index: false, follow: false },
};

export const viewport: Viewport = {
	themeColor: "#FAFAFA",
};

export default function DesignLayout({ children }: PropsWithChildren) {
	return <div className="design-shell">{children}</div>;
}
