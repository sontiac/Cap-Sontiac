"use client";

import { Button } from "@cap/ui";
import { useDetectPlatform } from "hooks/useDetectPlatform";
import { ArrowRight } from "lucide-react";
import Link from "next/link";
import { getPlatformIcon } from "@/utils/platform";
import { homepageCopy } from "../data/homepage-copy";
import UpgradeToPro from "./pages/_components/UpgradeToPro";

export function ReadyToGetStarted() {
	const { platform } = useDetectPlatform();
	const loading = platform === null;

	return (
		<div className="my-[120px] md:my-[180px] lg:my-[220px] mx-auto w-[calc(100%-20px)] max-w-[980px] px-5">
			<div className="flex flex-col items-center text-center">
				<span className="mb-6 inline-flex items-center gap-2 rounded-full border border-gray-4 bg-white px-3 py-1 text-[11px] font-medium tracking-wide text-gray-10 uppercase">
					<span className="size-1.5 rounded-full bg-gray-12" />
					Ready when you are
				</span>
				<h2 className="text-3xl font-medium leading-[1.1] tracking-[-0.02em] text-gray-12 md:text-5xl lg:text-6xl">
					{homepageCopy.readyToGetStarted.title}
				</h2>
				<p className="mt-5 max-w-xl text-base text-gray-10 md:text-lg">
					Free for personal use. Upgrade when you need more.
				</p>
				<div className="mt-10 flex flex-col items-center gap-3 sm:flex-row sm:gap-2">
					<Button
						variant="dark"
						href="/pricing"
						size="md"
						className="!h-11 !px-5 !text-[14px] font-medium"
					>
						{!loading && getPlatformIcon(platform)}
						{homepageCopy.readyToGetStarted.buttons.secondary}
						<ArrowRight className="size-4" />
					</Button>
					<UpgradeToPro text={homepageCopy.header.cta.primaryButton} />
				</div>
				<p className="mt-6 text-[13px] text-gray-9">
					or,{" "}
					<Link
						href="/loom-alternative"
						className="text-gray-10 underline-offset-4 hover:text-gray-12 hover:underline"
					>
						switch from Loom
					</Link>
				</p>
			</div>
		</div>
	);
}
