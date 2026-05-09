import type { MetadataRoute } from "next";
import { headers } from "next/headers";
import { seoPages } from "@/lib/seo-pages";
import {
	SOCIAL_CRAWLER_ROBOTS_USER_AGENTS,
	SOCIAL_REFERRER_DOMAINS,
} from "@/lib/social-crawlers";

export default async function robots(): Promise<MetadataRoute.Robots> {
	const seoPageSlugs = Object.keys(seoPages);
	const headersList = await headers();
	const referrer =
		headersList.get("x-referrer") || headersList.get("referer") || "";

	const isAllowedReferrer = SOCIAL_REFERRER_DOMAINS.some((domain) =>
		referrer.includes(domain),
	);

	const disallowPaths = [
		"/dashboard",
		"/login",
		"/invite",
		"/onboarding",
		"/record",
		"/home",
	];
	const searchDisallowPaths = [...disallowPaths];

	if (!isAllowedReferrer) {
		searchDisallowPaths.push("/s/*");
	}

	return {
		rules: [
			...SOCIAL_CRAWLER_ROBOTS_USER_AGENTS.map((userAgent) => ({
				userAgent,
				allow: ["/", "/s/*"],
				disallow: disallowPaths,
			})),
			{
				userAgent: "*",
				allow: [
					"/",
					"/blog/",
					...seoPageSlugs.map((slug) => `/${slug}`),
					...(isAllowedReferrer ? ["/s/*"] : []),
				],
				disallow: searchDisallowPaths,
			},
		],
		sitemap: "https://cap.so/sitemap.xml",
	};
}
