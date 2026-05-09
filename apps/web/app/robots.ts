import type { MetadataRoute } from "next";
import { seoPages } from "@/lib/seo-pages";
import { SOCIAL_CRAWLER_ROBOTS_USER_AGENTS } from "@/lib/social-crawlers";

export default async function robots(): Promise<MetadataRoute.Robots> {
	const seoPageSlugs = Object.keys(seoPages);

	const disallowPaths = [
		"/dashboard",
		"/login",
		"/invite",
		"/onboarding",
		"/record",
		"/home",
	];

	return {
		rules: [
			...SOCIAL_CRAWLER_ROBOTS_USER_AGENTS.map((userAgent) => ({
				userAgent,
				allow: ["/", "/s/*"],
				disallow: disallowPaths,
			})),
			{
				userAgent: "*",
				allow: ["/", "/blog/", ...seoPageSlugs.map((slug) => `/${slug}`)],
				disallow: disallowPaths,
			},
		],
		sitemap: "https://cap.so/sitemap.xml",
	};
}
