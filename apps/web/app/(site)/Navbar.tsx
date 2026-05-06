"use client";

import {
	Button,
	Logo,
	NavigationMenu,
	NavigationMenuContent,
	NavigationMenuItem,
	NavigationMenuLink,
	NavigationMenuList,
	NavigationMenuTrigger,
	navigationMenuTriggerStyle,
} from "@cap/ui";
import { classNames } from "@cap/utils";
import { Clapperboard, Zap } from "lucide-react";
import { motion } from "motion/react";
import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Suspense, useEffect, useState } from "react";
import MobileMenu from "@/components/ui/MobileMenu";
import { useCurrentUser } from "../Layout/AuthContext";

const Links = [
	{
		label: "Product",
		dropdown: [
			{
				label: "Instant Mode",
				sub: "Quick recordings with instant shareable links",
				href: "/features/instant-mode",
				icon: <Zap className="size-4 text-gray-12" strokeWidth={1.5} />,
			},
			{
				label: "Studio Mode",
				sub: "Professional recordings with advanced editing",
				href: "/features/studio-mode",
				icon: (
					<Clapperboard className="size-4 text-gray-12" strokeWidth={1.5} />
				),
			},
			{
				label: "Download App",
				sub: "Downloads for macOS & Windows",
				href: "/download",
			},
			{
				label: "Open Source",
				sub: "Cap is open source and available on GitHub",
				href: "https://github.com/CapSoftware/Cap",
			},
			{
				label: "Self-host Cap",
				sub: "Self-host Cap on your own infrastructure",
				href: "/self-hosting",
			},
			{
				label: "Join the community",
				sub: "Join the Cap community on Discord",
				href: "https://cap.link/discord",
			},
		],
	},
	{
		label: "Download",
		href: "/download",
	},
	{
		label: "Testimonials",
		href: "/testimonials",
	},
	{
		label: "Help",
		dropdown: [
			{
				label: "Documentation",
				sub: "Documentation for using Cap",
				href: "/docs",
			},
			{
				label: "FAQs",
				sub: "Frequently asked questions about Cap",
				href: "/faq",
			},
			{
				label: "Help Center",
				sub: "Guides, tutorials, and more. Currently in progress.",
				href: "https://help.cap.so",
			},
			{
				label: "Chat support",
				sub: "Support via chat",
				href: "https://discord.gg/y8gdQ3WRN3",
			},
		],
	},
	{
		label: "About",
		href: "/about",
	},
	{
		label: "Blog",
		href: "/blog",
	},
	{
		label: "Pricing",
		href: "/pricing",
	},
];

interface NavbarProps {
	stars?: string;
}

export const Navbar = ({ stars }: NavbarProps) => {
	const pathname = usePathname();
	const [showMobileMenu, setShowMobileMenu] = useState(false);
	const auth = useCurrentUser();

	const [scrolled, setScrolled] = useState(false);

	useEffect(() => {
		const onScroll = () => {
			setScrolled(window.scrollY > 4);
		};
		onScroll();
		document.addEventListener("scroll", onScroll, { passive: true });
		return () => {
			document.removeEventListener("scroll", onScroll);
		};
	}, []);

	return (
		<>
			<header
				className={classNames(
					"fixed left-0 right-0 top-0 z-[51] transition-colors duration-200",
					scrolled
						? "border-b border-gray-4 bg-[#FAFAFA]/85 backdrop-blur-md"
						: "border-b border-transparent bg-transparent",
				)}
			>
				<nav className="mx-auto flex h-14 w-full max-w-[1180px] items-center justify-between px-5 md:h-16 lg:px-8">
					<div className="flex items-center gap-10">
						<Link
							passHref
							href="/home"
							className="flex items-center"
							aria-label="Cap home"
						>
							<Logo
								hideLogoName={false}
								className="transition-opacity duration-200 hover:opacity-80"
								viewBoxDimensions="0 0 120 40"
								style={{ width: 84, height: 36 }}
							/>
						</Link>
						<div className="hidden lg:flex">
							<NavigationMenu>
								<NavigationMenuList className="space-x-0">
									{Links.map((link) => (
										<NavigationMenuItem key={link.label}>
											{link.dropdown ? (
												<>
													<NavigationMenuTrigger className="px-3 py-0 text-[13px] font-medium text-gray-10 hover:text-gray-12 active:text-gray-12 focus:text-gray-12 data-[state=open]:text-gray-12">
														{link.label}
													</NavigationMenuTrigger>
													<NavigationMenuContent>
														<ul className="grid gap-1 p-3 md:w-[400px] lg:w-[480px] lg:grid-cols-2">
															{link.dropdown.map((sublink) => (
																<li key={sublink.href}>
																	<NavigationMenuLink asChild>
																		<a
																			href={sublink.href}
																			className="block rounded-lg p-3 leading-tight no-underline outline-none transition-colors duration-200 select-none hover:bg-gray-3"
																		>
																			<div className="flex items-center gap-2 text-[13px] font-medium text-gray-12">
																				{sublink.icon && sublink.icon}
																				<span>{sublink.label}</span>
																			</div>
																			<p className="mt-1 line-clamp-2 text-[12px] leading-snug text-gray-10">
																				{sublink.sub}
																			</p>
																		</a>
																	</NavigationMenuLink>
																</li>
															))}
														</ul>
													</NavigationMenuContent>
												</>
											) : (
												<NavigationMenuLink asChild>
													<Link
														href={link.href}
														className={classNames(
															navigationMenuTriggerStyle(),
															pathname === link.href
																? "text-gray-12"
																: "text-gray-10",
															"px-3 py-0 text-[13px] font-medium hover:text-gray-12",
														)}
													>
														{link.label}
													</Link>
												</NavigationMenuLink>
											)}
										</NavigationMenuItem>
									))}
								</NavigationMenuList>
							</NavigationMenu>
						</div>
					</div>
					<div className="hidden items-center gap-2 lg:flex">
						<Button
							variant="ghost"
							icon={
								<Image src="/github.svg" alt="Github" width={14} height={14} />
							}
							target="_blank"
							href="https://github.com/CapSoftware/Cap"
							size="sm"
							className="!h-9 !px-3 !text-[13px] !text-gray-11 hover:!text-gray-12"
						>
							{`${stars ? stars : "GitHub"}`}
						</Button>
						<Suspense
							fallback={
								<Button
									variant="dark"
									disabled
									size="sm"
									className="!h-9 !px-4 !text-[13px]"
								>
									Loading…
								</Button>
							}
						>
							{!auth && (
								<Button
									variant="ghost"
									href="/login"
									size="sm"
									className="!h-9 !px-3 !text-[13px] !text-gray-11 hover:!text-gray-12"
								>
									Login
								</Button>
							)}
							<LoginOrDashboard />
						</Suspense>
					</div>
					<button
						type="button"
						className="flex lg:hidden"
						onClick={() => setShowMobileMenu(!showMobileMenu)}
						aria-label="Toggle menu"
					>
						<div className="mr-1 flex flex-col gap-[5px]">
							<motion.div
								initial={{ opacity: 1 }}
								animate={{
									rotate: showMobileMenu ? 45 : 0,
									y: showMobileMenu ? 7 : 0,
								}}
								transition={{ duration: 0.2 }}
								className="h-0.5 w-6 bg-gray-12"
							/>
							<motion.div
								initial={{ opacity: 1 }}
								animate={{
									opacity: showMobileMenu ? 0 : 1,
									x: showMobileMenu ? -5 : 0,
								}}
								transition={{ duration: 0.2 }}
								className="h-0.5 w-6 bg-gray-12"
							/>
							<motion.div
								initial={{ opacity: 1 }}
								animate={{
									rotate: showMobileMenu ? -45 : 0,
									y: showMobileMenu ? -7 : 0,
								}}
								transition={{ duration: 0.2 }}
								className="h-0.5 w-6 bg-gray-12"
							/>
						</div>
					</button>
				</nav>
			</header>
			{showMobileMenu && (
				<MobileMenu
					setShowMobileMenu={setShowMobileMenu}
					auth={auth}
					stars={stars}
				/>
			)}
		</>
	);
};

function LoginOrDashboard() {
	const auth = useCurrentUser();

	return (
		<Button
			variant="dark"
			href={auth ? "/dashboard" : "/signup"}
			size="sm"
			className="!h-9 !px-4 !text-[13px]"
		>
			{auth ? "Dashboard" : "Sign up"}
		</Button>
	);
}
