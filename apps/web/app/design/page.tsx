import { Button } from "@cap/ui";
import {
	ArrowRight,
	ArrowUpRight,
	Check,
	Command,
	Sparkles,
} from "lucide-react";
import Link from "next/link";

type SectionId =
	| "foundations"
	| "typography"
	| "color"
	| "buttons"
	| "surfaces"
	| "patterns";

interface SectionMeta {
	id: SectionId;
	label: string;
	eyebrow: string;
	title: string;
	description: string;
}

const SECTIONS: SectionMeta[] = [
	{
		id: "foundations",
		label: "Foundations",
		eyebrow: "01 — Foundations",
		title: "Quiet by default",
		description:
			"A minimal, light surface with restrained accents. Type and space do most of the work; color is reserved for meaning.",
	},
	{
		id: "typography",
		label: "Typography",
		eyebrow: "02 — Typography",
		title: "Type scale",
		description:
			"A single sans typeface with tight, intentional sizing. Display sets the tone; body stays comfortable to read.",
	},
	{
		id: "color",
		label: "Color",
		eyebrow: "03 — Color",
		title: "A monochrome palette",
		description:
			"Twelve steps of gray, plus a single utility blue for links and focus. No decorative color.",
	},
	{
		id: "buttons",
		label: "Buttons",
		eyebrow: "04 — Buttons",
		title: "Buttons & controls",
		description:
			"A near-black primary, a hairline outline, and a quiet ghost. Everything is round; nothing shouts.",
	},
	{
		id: "surfaces",
		label: "Surfaces",
		eyebrow: "05 — Surfaces",
		title: "Cards, inputs & pills",
		description:
			"Surfaces lift gently above the page with hairline borders and almost no shadow.",
	},
	{
		id: "patterns",
		label: "Patterns",
		eyebrow: "06 — Patterns",
		title: "Patterns in context",
		description:
			"A handful of compositions you can lift directly. Use them as starting points; never as templates.",
	},
];

const SECTION_BODIES: Record<SectionId, () => React.ReactElement> = {
	foundations: Foundations,
	typography: Typography,
	color: Color,
	buttons: Buttons,
	surfaces: Surfaces,
	patterns: Patterns,
};

export default function DesignPage() {
	return (
		<main className="min-h-screen bg-[#FAFAFA] text-gray-12 antialiased selection:bg-gray-12 selection:text-white">
			<DesignNav />
			<div className="mx-auto max-w-[1120px] px-6 pt-16 pb-32 md:px-10 md:pt-24 md:pb-40">
				<Intro />

				{SECTIONS.map((section) => {
					const Body = SECTION_BODIES[section.id];
					return (
						<Section
							key={section.id}
							anchor={section.id}
							eyebrow={section.eyebrow}
							title={section.title}
							description={section.description}
						>
							<Body />
						</Section>
					);
				})}
			</div>
			<Footer />
		</main>
	);
}

function DesignNav() {
	return (
		<nav className="sticky top-0 z-30 w-full border-b border-gray-4 bg-[#FAFAFA]/80 backdrop-blur-md">
			<div className="mx-auto flex h-14 max-w-[1120px] items-center justify-between px-6 md:px-10">
				<Link
					href="/"
					className="flex items-center gap-2 text-[13px] font-medium text-gray-12 hover:text-gray-12"
				>
					<span className="inline-block size-1.5 rounded-full bg-gray-12" />
					Cap Design
				</Link>
				<div className="hidden items-center gap-6 md:flex">
					{SECTIONS.map((s) => (
						<a
							key={s.id}
							href={`#${s.id}`}
							className="text-[13px] text-gray-10 transition-colors hover:text-gray-12"
						>
							{s.label}
						</a>
					))}
				</div>
				<a
					href="https://github.com/CapSoftware/Cap"
					target="_blank"
					rel="noreferrer"
					className="hidden items-center gap-1 text-[13px] text-gray-10 transition-colors hover:text-gray-12 md:inline-flex"
				>
					Source <ArrowUpRight className="size-3.5" />
				</a>
			</div>
		</nav>
	);
}

function Intro() {
	return (
		<header className="border-b border-gray-4 pb-16 md:pb-24">
			<p className="mb-6 inline-flex items-center gap-2 rounded-full border border-gray-4 bg-white px-3 py-1 text-[11px] font-medium tracking-wide text-gray-10 uppercase">
				<span className="size-1.5 rounded-full bg-gray-12" />
				Living document — v1.0
			</p>
			<h1 className="text-4xl font-medium leading-[1.05] tracking-[-0.02em] text-gray-12 md:text-6xl lg:text-[80px]">
				The Cap design system.
			</h1>
			<p className="mt-6 max-w-xl text-base text-gray-10 md:text-lg">
				A quiet, monochrome system built around clean type, thoughtful spacing
				and one calm accent. Designed to get out of the way of the product.
			</p>
			<div className="mt-10 flex flex-wrap items-center gap-3">
				<Button href="#foundations" variant="dark" size="md">
					Start exploring
					<ArrowRight className="size-4" />
				</Button>
				<Button href="#patterns" variant="outline" size="md">
					Jump to patterns
				</Button>
			</div>
		</header>
	);
}

function Section({
	anchor,
	eyebrow,
	title,
	description,
	children,
}: {
	anchor: string;
	eyebrow: string;
	title: string;
	description: string;
	children: React.ReactNode;
}) {
	return (
		<section
			id={anchor}
			className="grid gap-10 border-b border-gray-4 py-16 md:grid-cols-[260px_1fr] md:gap-16 md:py-24"
		>
			<div className="md:sticky md:top-24 md:self-start">
				<p className="mb-3 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					{eyebrow}
				</p>
				<h2 className="text-2xl font-medium tracking-[-0.01em] text-gray-12 md:text-[28px]">
					{title}
				</h2>
				<p className="mt-3 text-sm leading-relaxed text-gray-10">
					{description}
				</p>
			</div>
			<div className="min-w-0">{children}</div>
		</section>
	);
}

function Foundations() {
	const principles = [
		{
			title: "Calm",
			body: "Light surfaces, hairline borders, restrained motion. The product should feel still until it needs not to.",
		},
		{
			title: "Monochrome",
			body: "Grayscale by default. Color appears only when it carries meaning — links, focus, status.",
		},
		{
			title: "Typographic",
			body: "Clear hierarchy through size and weight, never through ornament. Text is the primary material.",
		},
		{
			title: "Generous",
			body: "Whitespace is a first-class element. Density is earned, not assumed.",
		},
	];
	return (
		<div className="grid gap-3 sm:grid-cols-2">
			{principles.map((p) => (
				<div
					key={p.title}
					className="rounded-2xl border border-gray-4 bg-white p-6"
				>
					<p className="text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
						Principle
					</p>
					<h3 className="mt-3 text-lg font-medium text-gray-12">{p.title}</h3>
					<p className="mt-2 text-sm leading-relaxed text-gray-10">{p.body}</p>
				</div>
			))}
		</div>
	);
}

const TYPE_SCALE = [
	{
		name: "Display",
		className: "text-[56px] md:text-[80px] leading-[1.02] tracking-[-0.02em]",
		sample: "Beautiful, shareable recordings.",
	},
	{
		name: "H1",
		className: "text-4xl md:text-5xl leading-[1.1] tracking-[-0.015em]",
		sample: "One app, every workflow.",
	},
	{
		name: "H2",
		className: "text-2xl md:text-3xl leading-[1.2] tracking-[-0.01em]",
		sample: "Built for how you actually work.",
	},
	{
		name: "H3",
		className: "text-xl leading-[1.3]",
		sample: "Studio Mode",
	},
	{
		name: "Body",
		className: "text-[17px] leading-[1.6] text-gray-11",
		sample:
			"Cap is the open source alternative to Loom. Lightweight, powerful, cross-platform.",
	},
	{
		name: "Body S",
		className: "text-sm leading-[1.55] text-gray-10",
		sample: "No credit card required. Get started for free.",
	},
	{
		name: "Caption",
		className:
			"text-[11px] tracking-[0.18em] uppercase text-gray-9 font-medium",
		sample: "Open source · MIT",
	},
];

function Typography() {
	return (
		<div className="divide-y divide-gray-4 rounded-2xl border border-gray-4 bg-white">
			{TYPE_SCALE.map((row) => (
				<div
					key={row.name}
					className="grid gap-3 p-6 md:grid-cols-[110px_1fr] md:gap-8 md:p-8"
				>
					<div className="text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
						{row.name}
					</div>
					<div className={`min-w-0 text-gray-12 ${row.className}`}>
						{row.sample}
					</div>
				</div>
			))}
		</div>
	);
}

const GRAY_STEPS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] as const;

function Color() {
	return (
		<div className="space-y-8">
			<div>
				<p className="mb-3 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Gray — surfaces & text
				</p>
				<div className="grid grid-cols-6 gap-2 rounded-2xl border border-gray-4 bg-white p-4 md:grid-cols-12">
					{GRAY_STEPS.map((step) => (
						<div key={step} className="flex flex-col items-stretch gap-2">
							<div
								className="aspect-square w-full rounded-lg border border-gray-4"
								style={{ background: `var(--gray-${step})` }}
							/>
							<p className="text-center text-[10px] font-medium text-gray-10">
								{step}
							</p>
						</div>
					))}
				</div>
			</div>
			<div>
				<p className="mb-3 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Accent — used sparingly
				</p>
				<div className="grid gap-3 sm:grid-cols-3">
					<AccentSwatch
						label="Ink"
						hint="Primary text & buttons"
						color="var(--gray-12)"
						textColor="#fff"
					/>
					<AccentSwatch
						label="Paper"
						hint="Surfaces"
						color="#FFFFFF"
						textColor="var(--gray-12)"
						bordered
					/>
					<AccentSwatch
						label="Signal"
						hint="Links & focus only"
						color="var(--blue-9)"
						textColor="#fff"
					/>
				</div>
			</div>
		</div>
	);
}

function AccentSwatch({
	label,
	hint,
	color,
	textColor,
	bordered,
}: {
	label: string;
	hint: string;
	color: string;
	textColor: string;
	bordered?: boolean;
}) {
	return (
		<div
			className={`rounded-2xl ${bordered ? "border border-gray-4" : ""} overflow-hidden`}
		>
			<div className="flex h-32 items-end p-5" style={{ background: color }}>
				<span className="text-[13px] font-medium" style={{ color: textColor }}>
					{label}
				</span>
			</div>
			<div className="flex items-center justify-between border-t border-gray-4 bg-white px-5 py-3">
				<span className="text-[13px] text-gray-12">{hint}</span>
				<span className="text-[11px] font-mono text-gray-9">{color}</span>
			</div>
		</div>
	);
}

function Buttons() {
	return (
		<div className="space-y-3">
			<div className="rounded-2xl border border-gray-4 bg-white p-6 md:p-8">
				<p className="mb-5 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Variants
				</p>
				<div className="flex flex-wrap gap-3">
					<Button variant="dark" size="md">
						Get started
					</Button>
					<Button variant="outline" size="md">
						Learn more
					</Button>
					<Button variant="white" size="md">
						Secondary
					</Button>
					<Button variant="ghost" size="md" className="!text-gray-12">
						Ghost
					</Button>
					<Button variant="destructive" size="md">
						Delete
					</Button>
				</div>
			</div>
			<div className="rounded-2xl border border-gray-4 bg-white p-6 md:p-8">
				<p className="mb-5 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Sizes
				</p>
				<div className="flex flex-wrap items-center gap-3">
					<Button variant="dark" size="xs">
						Extra small
					</Button>
					<Button variant="dark" size="sm">
						Small
					</Button>
					<Button variant="dark" size="md">
						Medium
					</Button>
					<Button variant="dark" size="lg">
						Large
					</Button>
				</div>
			</div>
			<div className="rounded-2xl border border-gray-4 bg-white p-6 md:p-8">
				<p className="mb-5 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					With icon & shortcut
				</p>
				<div className="flex flex-wrap items-center gap-3">
					<Button
						variant="dark"
						size="md"
						icon={<Sparkles className="size-4" />}
					>
						Try Cap AI
					</Button>
					<Button variant="outline" size="md" kbd="K">
						<Command className="size-4" /> Search
					</Button>
				</div>
			</div>
		</div>
	);
}

function Surfaces() {
	return (
		<div className="grid gap-3 md:grid-cols-2">
			<SurfaceCard
				title="Card"
				body="Hairline border, white surface, generous padding. The default container."
			/>
			<SurfaceCard
				title="Card · raised"
				body="A whisper of shadow when the surface needs to feel slightly lifted."
				raised
			/>
			<div className="rounded-2xl border border-gray-4 bg-white p-6">
				<p className="mb-3 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Input
				</p>
				<input
					type="email"
					placeholder="you@company.com"
					className="block w-full rounded-full border border-gray-4 bg-white px-4 py-2.5 text-sm text-gray-12 placeholder:text-gray-9 transition-colors focus:border-gray-12 focus:outline-none"
				/>
				<div className="mt-4 flex flex-wrap items-center gap-2">
					<Pill>Default</Pill>
					<Pill tone="ink">Ink</Pill>
					<Pill icon={<Check className="size-3" />}>Verified</Pill>
				</div>
			</div>
			<div className="rounded-2xl border border-gray-4 bg-white p-6">
				<p className="mb-3 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Stat
				</p>
				<div className="grid grid-cols-3 gap-4">
					<Stat label="Users" value="30k+" />
					<Stat label="GitHub" value="9.4k" />
					<Stat label="Open issues" value="42" />
				</div>
			</div>
		</div>
	);
}

function SurfaceCard({
	title,
	body,
	raised,
}: {
	title: string;
	body: string;
	raised?: boolean;
}) {
	return (
		<div
			className={`rounded-2xl border border-gray-4 bg-white p-6 ${
				raised
					? "shadow-[0_1px_0_0_rgba(0,0,0,0.02),0_8px_24px_-12px_rgba(0,0,0,0.08)]"
					: ""
			}`}
		>
			<p className="mb-3 text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
				{title}
			</p>
			<h4 className="text-lg font-medium text-gray-12">{body}</h4>
			<p className="mt-2 text-sm leading-relaxed text-gray-10">
				Use cards to group related content. Avoid stacking shadows; let the
				border do the work.
			</p>
		</div>
	);
}

function Pill({
	children,
	icon,
	tone = "default",
}: {
	children: React.ReactNode;
	icon?: React.ReactNode;
	tone?: "default" | "ink";
}) {
	const toneClass =
		tone === "ink"
			? "bg-gray-12 text-white border-gray-12"
			: "bg-white text-gray-12 border-gray-4";
	return (
		<span
			className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-medium ${toneClass}`}
		>
			{icon}
			{children}
		</span>
	);
}

function Stat({ label, value }: { label: string; value: string }) {
	return (
		<div>
			<p className="text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
				{label}
			</p>
			<p className="mt-2 text-2xl font-medium tracking-[-0.01em] text-gray-12">
				{value}
			</p>
		</div>
	);
}

function Patterns() {
	return (
		<div className="space-y-6">
			<HeroPattern />
			<FeatureRowPattern />
			<CalloutPattern />
		</div>
	);
}

function HeroPattern() {
	return (
		<div className="relative overflow-hidden rounded-2xl border border-gray-4 bg-white p-8 md:p-14">
			<p className="mb-5 inline-flex items-center gap-2 rounded-full border border-gray-4 bg-white px-3 py-1 text-[11px] font-medium tracking-wide text-gray-10 uppercase">
				<span className="size-1.5 rounded-full bg-gray-12" /> Hero
			</p>
			<h3 className="text-3xl font-medium leading-[1.05] tracking-[-0.02em] text-gray-12 md:text-5xl">
				Beautiful, shareable
				<br className="hidden md:block" /> screen recordings.
			</h3>
			<p className="mt-4 max-w-lg text-base text-gray-10 md:text-lg">
				Lightweight, open source, cross-platform. Record and share securely in
				seconds.
			</p>
			<div className="mt-7 flex flex-wrap gap-3">
				<Button variant="dark" size="md">
					Download for macOS
					<ArrowRight className="size-4" />
				</Button>
				<Button variant="outline" size="md">
					See pricing
				</Button>
			</div>
		</div>
	);
}

function FeatureRowPattern() {
	const items = [
		{
			title: "Instant",
			body: "Hit record, stop, share link. Live in seconds.",
		},
		{ title: "Studio", body: "Local-first quality with a full editor." },
		{ title: "Screenshot", body: "Capture and annotate in one shortcut." },
	];
	return (
		<div className="rounded-2xl border border-gray-4 bg-white">
			<div className="border-b border-gray-4 px-6 py-5 md:px-8">
				<p className="text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
					Three modes
				</p>
				<h3 className="mt-1 text-2xl font-medium tracking-[-0.01em] text-gray-12">
					One app, every workflow.
				</h3>
			</div>
			<div className="grid divide-gray-4 md:grid-cols-3 md:divide-x">
				{items.map((item, i) => (
					<div
						key={item.title}
						className={`p-6 md:p-8 ${i !== items.length - 1 ? "border-b border-gray-4 md:border-b-0" : ""}`}
					>
						<p className="text-[11px] font-medium tracking-[0.18em] text-gray-9 uppercase">
							0{i + 1}
						</p>
						<h4 className="mt-3 text-lg font-medium text-gray-12">
							{item.title}
						</h4>
						<p className="mt-2 text-sm leading-relaxed text-gray-10">
							{item.body}
						</p>
					</div>
				))}
			</div>
		</div>
	);
}

function CalloutPattern() {
	return (
		<div className="relative overflow-hidden rounded-2xl border border-gray-12 bg-gray-12 p-8 text-white md:p-14">
			<p className="text-[11px] font-medium tracking-[0.18em] text-gray-7 uppercase">
				Ready when you are
			</p>
			<h3 className="mt-3 text-2xl font-medium tracking-[-0.01em] text-white md:text-4xl">
				Start recording in under a minute.
			</h3>
			<div className="mt-7 flex flex-wrap gap-3">
				<Button
					variant="white"
					size="md"
					className="!bg-white !text-gray-12 hover:!bg-gray-3"
				>
					Download
					<ArrowRight className="size-4" />
				</Button>
				<Button
					variant="ghost"
					size="md"
					className="!text-white hover:!bg-white/10"
				>
					See pricing
				</Button>
			</div>
		</div>
	);
}

function Footer() {
	return (
		<footer className="border-t border-gray-4 bg-[#FAFAFA]">
			<div className="mx-auto flex max-w-[1120px] flex-col items-start justify-between gap-4 px-6 py-10 text-[13px] text-gray-10 md:flex-row md:items-center md:px-10">
				<p>Cap Design — a quiet system, in service of the product.</p>
				<Link href="/" className="hover:text-gray-12">
					Back to cap.so →
				</Link>
			</div>
		</footer>
	);
}
