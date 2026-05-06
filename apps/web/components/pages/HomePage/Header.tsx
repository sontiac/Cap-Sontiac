"use client";

import { Button } from "@cap/ui";
import { faPlay } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { AnimatePresence, motion } from "framer-motion";
import { useDetectPlatform } from "hooks/useDetectPlatform";
import { ArrowRight } from "lucide-react";
import Image from "next/image";
import Link from "next/link";
import { useState, useTransition } from "react";
import { sendDownloadLink } from "@/actions/send-download-link";
import { trackEvent } from "@/app/utils/analytics";
import { LogoMarquee } from "@/components/ui/LogoMarquee";
import {
	getDownloadButtonText,
	getDownloadUrl,
	getPlatformIcon,
} from "@/utils/platform";
import { homepageCopy } from "../../../data/homepage-copy";
import VideoModal from "./VideoModal";

interface HeaderProps {
	serverHomepageCopyVariant?: string;
}

const fadeIn = {
	hidden: { opacity: 0, y: 16 },
	visible: (custom: number) => ({
		opacity: 1,
		y: 0,
		transition: {
			delay: custom * 0.08,
			duration: 0.5,
			ease: "easeOut",
		},
	}),
};

const Header = ({ serverHomepageCopyVariant = "" }: HeaderProps) => {
	const [videoToggled, setVideoToggled] = useState(false);
	const { platform, isIntel } = useDetectPlatform();
	const loading = platform === null;
	const [email, setEmail] = useState("");
	const [emailStatus, setEmailStatus] = useState<
		"idle" | "sending" | "sent" | "error"
	>("idle");
	const [emailError, setEmailError] = useState("");
	const [isPending, startTransition] = useTransition();
	const primaryDownloadUrl =
		platform === "windows" ? "/download" : getDownloadUrl(platform, isIntel);

	const handleEmailSubmit = (e: React.FormEvent) => {
		e.preventDefault();
		setEmailStatus("sending");
		setEmailError("");

		startTransition(async () => {
			const result = await sendDownloadLink(email);
			if (result.success) {
				setEmailStatus("sent");
				if (typeof window !== "undefined" && window.bento) {
					window.bento.identify(email);
				}
			} else {
				setEmailStatus("error");
				setEmailError(result.error ?? "Something went wrong.");
			}
		});
	};

	const variant =
		serverHomepageCopyVariant as keyof typeof homepageCopy.header.variants;
	const headerContent =
		homepageCopy.header.variants[variant] ||
		homepageCopy.header.variants.default;

	return (
		<section className="relative w-full overflow-hidden">
			<div className="mx-auto w-full max-w-[1180px] px-5 pt-28 pb-16 md:pt-36 md:pb-24 lg:px-8">
				<div className="mx-auto flex max-w-[860px] flex-col items-center text-center">
					<motion.span
						initial="hidden"
						animate="visible"
						custom={0}
						variants={fadeIn}
						className="mb-7 inline-flex items-center gap-2 rounded-full border border-gray-4 bg-white px-3 py-1 text-[11px] font-medium tracking-wide text-gray-10 uppercase"
					>
						<span className="size-1.5 rounded-full bg-gray-12" />
						Open source · Cross-platform
					</motion.span>

					<motion.h1
						initial="hidden"
						animate="visible"
						custom={1}
						variants={fadeIn}
						className="text-[2.5rem] leading-[1.05] tracking-[-0.02em] font-medium text-gray-12 md:text-[64px] lg:text-[80px]"
					>
						{headerContent.title}
					</motion.h1>

					<motion.p
						initial="hidden"
						animate="visible"
						custom={2}
						variants={fadeIn}
						className="mx-auto mt-6 max-w-[640px] text-base leading-relaxed text-gray-10 md:text-lg"
					>
						{headerContent.description}
					</motion.p>

					<motion.div
						initial="hidden"
						animate="visible"
						custom={3}
						variants={fadeIn}
						className="mt-10 hidden flex-wrap items-center justify-center gap-3 md:flex"
					>
						<Button
							variant="dark"
							href={primaryDownloadUrl}
							onClick={() =>
								trackEvent("download_cta_clicked", {
									source_page: "home_header",
									cta_location: "primary",
									target_url: primaryDownloadUrl,
									detected_platform: platform ?? "unknown",
									is_intel: Boolean(isIntel),
								})
							}
							size="md"
							className="!h-11 !px-5 !text-[14px] font-medium"
						>
							{!loading && getPlatformIcon(platform)}
							{getDownloadButtonText(platform, loading, isIntel)}
						</Button>
						<Button
							variant="outline"
							href="/pricing"
							size="md"
							className="!h-11 !px-5 !text-[14px] font-medium"
						>
							See pricing
							<ArrowRight className="size-4" />
						</Button>
					</motion.div>

					<motion.div
						initial="hidden"
						animate="visible"
						custom={3}
						variants={fadeIn}
						className="mt-10 flex w-full max-w-sm flex-col gap-3 md:hidden"
					>
						{emailStatus === "sent" ? (
							<div className="rounded-2xl border border-gray-4 bg-white px-4 py-3 text-left">
								<p className="text-sm font-medium text-gray-12">
									Check your inbox.
								</p>
								<p className="mt-1 text-xs text-gray-10">
									We've sent the download links to{" "}
									<strong className="text-gray-12">{email}</strong>.
								</p>
							</div>
						) : (
							<form
								onSubmit={handleEmailSubmit}
								className="flex flex-col gap-2"
							>
								<input
									type="email"
									value={email}
									onChange={(e) => setEmail(e.target.value)}
									placeholder="you@email.com"
									required
									className="w-full rounded-full border border-gray-4 bg-white px-4 py-2.5 text-sm text-gray-12 placeholder:text-gray-9 focus:border-gray-12 focus:outline-none"
								/>
								<button
									type="submit"
									disabled={isPending}
									className="w-full rounded-full bg-gray-12 px-5 py-2.5 text-sm font-medium text-white transition-colors hover:bg-gray-11 disabled:opacity-60"
								>
									{isPending ? "Sending…" : "Email me the download link"}
								</button>
								{emailStatus === "error" && (
									<p className="text-xs text-red-500">{emailError}</p>
								)}
							</form>
						)}
						<Link
							href="/pricing"
							className="text-center text-[13px] text-gray-10 underline-offset-4 hover:text-gray-12 hover:underline"
						>
							See pricing →
						</Link>
					</motion.div>

					<motion.p
						initial="hidden"
						animate="visible"
						custom={4}
						variants={fadeIn}
						className="mt-4 text-[13px] text-gray-9"
					>
						{homepageCopy.header.cta.freeVersionText} ·{" "}
						<Link
							href="/download"
							onClick={() =>
								trackEvent("download_cta_clicked", {
									source_page: "home_header",
									cta_location: "see_other_options",
									target_url: "/download",
									detected_platform: platform ?? "unknown",
									is_intel: Boolean(isIntel),
								})
							}
							className="text-gray-10 underline-offset-4 hover:text-gray-12 hover:underline"
						>
							{homepageCopy.header.cta.seeOtherOptionsText}
						</Link>
					</motion.p>
				</div>

				<motion.div
					initial={{ opacity: 0, y: 32 }}
					animate={{ opacity: 1, y: 0 }}
					transition={{ delay: 0.55, duration: 0.7, ease: "easeOut" }}
					className="relative mx-auto mt-16 max-w-[1080px] md:mt-20"
				>
					<button
						type="button"
						onClick={() => setVideoToggled(true)}
						aria-label="Play product video"
						className="group relative block w-full overflow-hidden rounded-2xl border border-gray-4 bg-white shadow-[0_8px_32px_-8px_rgba(0,0,0,0.06)] transition-all hover:shadow-[0_12px_40px_-8px_rgba(0,0,0,0.10)]"
					>
						<Image
							src="/illustrations/app.webp"
							width={1920}
							height={1080}
							quality={100}
							alt="Cap app preview"
							className="block w-full object-cover"
						/>
						<span className="absolute inset-0 flex items-center justify-center bg-transparent transition-colors group-hover:bg-black/5">
							<span className="flex size-14 items-center justify-center rounded-full bg-white/95 shadow-md ring-1 ring-black/5 backdrop-blur-sm transition-transform group-hover:scale-105 md:size-16">
								<FontAwesomeIcon
									icon={faPlay}
									className="ml-0.5 size-5 text-gray-12 md:size-6"
								/>
							</span>
						</span>
					</button>
				</motion.div>

				<motion.div
					initial={{ opacity: 0 }}
					animate={{ opacity: 1 }}
					transition={{ delay: 0.7, duration: 0.6 }}
					className="mt-20 md:mt-28"
				>
					<p className="mb-6 text-center text-[12px] font-medium tracking-[0.18em] text-gray-9 uppercase">
						Trusted by 30,000+ teams, builders & creators
					</p>
					<LogoMarquee />
				</motion.div>
			</div>

			<AnimatePresence>
				{videoToggled && <VideoModal setVideoToggled={setVideoToggled} />}
			</AnimatePresence>
		</section>
	);
};

export default Header;
