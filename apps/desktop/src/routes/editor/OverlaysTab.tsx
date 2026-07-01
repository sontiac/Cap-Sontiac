import { Button } from "@cap/ui-solid";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appLocalDataDir, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { exists, mkdir, readDir } from "@tauri-apps/plugin-fs";
import { cx } from "cva";
import { createResource, For, Show } from "solid-js";

import { commands, type OverlayAnim, type OverlaySegment } from "~/utils/tauri";
import IconLucideImagePlus from "~icons/lucide/image-plus";
import IconLucideTrash2 from "~icons/lucide/trash-2";

import { serializeProjectConfiguration, useEditorContext } from "./context";

const OVERLAY_FOLDER = "overlays";
const OVERLAY_DEFAULT_DURATION = 3;
const OVERLAY_EXTENSIONS = /\.png$/i;

const ANIM_OPTIONS = [
	{ value: "", label: "None" },
	{ value: "slideLeft", label: "Slide left" },
	{ value: "slideRight", label: "Slide right" },
	{ value: "slideUp", label: "Slide up" },
	{ value: "slideDown", label: "Slide down" },
	{ value: "pop", label: "Pop" },
] as const;

type SegmentView = Required<Omit<OverlaySegment, "animIn" | "animOut">> & {
	animIn: OverlayAnim | null;
	animOut: OverlayAnim | null;
};

interface OverlayAsset {
	name: string;
	path: string;
}

async function ensureOverlayDir(): Promise<string> {
	const dir = await join(await appLocalDataDir(), OVERLAY_FOLDER);
	if (!(await exists(dir))) {
		await mkdir(dir, { recursive: true });
	}
	return dir;
}

async function listOverlayAssets(): Promise<OverlayAsset[]> {
	const dir = await ensureOverlayDir();
	const entries = await readDir(dir);
	return entries
		.filter((e) => e.isFile && OVERLAY_EXTENSIONS.test(e.name))
		.map((e) => ({ name: e.name, path: `${dir}/${e.name}` }));
}

function fileName(path: string): string {
	return path.split(/[\\/]/).pop() ?? path;
}

export function OverlaysTab() {
	const { project, setProject, setEditorState, editorState } =
		useEditorContext();

	let hasSeeded = false;
	const seedOnceThenList = async (): Promise<OverlayAsset[]> => {
		if (!hasSeeded) {
			hasSeeded = true;
			try {
				await commands.seedOverlays();
			} catch (err) {
				console.error("Failed to seed default overlays:", err);
			}
		}
		return listOverlayAssets();
	};
	const [assets] = createResource(seedOnceThenList, { initialValue: [] });

	const segments = (): SegmentView[] =>
		(project.timeline?.overlaySegments ?? []) as SegmentView[];

	const selectedIndex = (): number | null => {
		const s = editorState.timeline.selection;
		if (!s || s.type !== "overlay") return null;
		return s.indices[0] ?? null;
	};

	const selectSegment = (index: number) => {
		setEditorState("timeline", "selection", {
			type: "overlay",
			indices: [index],
		});
	};

	const clearSelectionIfPointingAt = (index: number) => {
		const current = selectedIndex();
		if (current === index) {
			setEditorState("timeline", "selection", null);
		}
	};

	const writeSegments = async (next: SegmentView[]) => {
		if (!project.timeline) return;
		const updated = {
			...project,
			timeline: {
				...project.timeline,
				overlaySegments: next,
			},
		} as typeof project;
		setProject(updated);
		await commands.setProjectConfig(serializeProjectConfiguration(updated));
	};

	const addOverlay = async (filePath: string) => {
		const time = editorState.previewTime ?? editorState.playbackTime;
		const newSegment: SegmentView = {
			id: crypto.randomUUID(),
			start: time,
			end: time + OVERLAY_DEFAULT_DURATION,
			filePath,
			center: { x: 0.5, y: 0.5 },
			size: { x: 0.5, y: 0.3 },
			opacity: 1,
			fadeDuration: 0.2,
			animIn: null,
			animOut: null,
			animDuration: 0.35,
		};
		const next = [...segments(), newSegment];
		await writeSegments(next);
		selectSegment(next.length - 1);
	};

	const pickCustomOverlay = async () => {
		const picked = await open({
			multiple: false,
			directory: false,
			filters: [{ name: "PNG image", extensions: ["png"] }],
		});
		if (typeof picked === "string") {
			await addOverlay(picked);
		}
	};

	const updateSegment = async (id: string, patch: Partial<SegmentView>) => {
		await writeSegments(
			segments().map((s) => (s.id === id ? { ...s, ...patch } : s)),
		);
	};

	const deleteSegment = async (index: number, id: string) => {
		clearSelectionIfPointingAt(index);
		await writeSegments(segments().filter((s) => s.id !== id));
	};

	return (
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<div class="flex items-center justify-between">
					<h3 class="text-sm font-medium text-gray-12">Overlay assets</h3>
					<Button variant="secondary" size="sm" onClick={pickCustomOverlay}>
						<IconLucideImagePlus class="size-4 mr-1" />
						Custom PNG
					</Button>
				</div>
				<Show
					when={(assets() ?? []).length > 0}
					fallback={
						<p class="text-xs text-gray-11">
							No bundled overlays found. Use “Custom PNG” to add a transparent
							PNG.
						</p>
					}
				>
					<div class="grid grid-cols-3 gap-2">
						<For each={assets() ?? []}>
							{(asset) => (
								<button
									type="button"
									onClick={() => addOverlay(asset.path)}
									class="flex flex-col items-center gap-1 rounded p-2 hover:bg-gray-3"
									title={`Add ${asset.name}`}
								>
									<img
										src={convertFileSrc(asset.path)}
										alt={asset.name}
										class="w-full h-16 object-contain"
									/>
									<span class="text-[0.65rem] truncate w-full text-center text-gray-11">
										{asset.name}
									</span>
								</button>
							)}
						</For>
					</div>
				</Show>
			</div>

			<div class="flex flex-col gap-2">
				<h3 class="text-sm font-medium text-gray-12">
					Overlays on timeline ({segments().length})
				</h3>
				<Show
					when={segments().length > 0}
					fallback={
						<p class="text-xs text-gray-11">
							Click an asset above to place it at the playhead.
						</p>
					}
				>
					<ul class="flex flex-col gap-3">
						<For each={segments()}>
							{(seg, index) => {
								const isSelected = () => selectedIndex() === index();
								return (
									<li
										class={cx(
											"flex flex-col gap-2 rounded border p-2 transition-colors cursor-pointer",
											isSelected()
												? "border-blue-9 bg-blue-3"
												: "border-gray-3 hover:border-gray-6",
										)}
										onClick={() => selectSegment(index())}
									>
										<div class="flex items-center gap-2">
											<img
												src={convertFileSrc(seg.filePath)}
												alt={fileName(seg.filePath)}
												class="size-8 object-contain shrink-0"
											/>
											<span
												class="flex-1 text-xs truncate text-gray-12"
												title={seg.filePath}
											>
												{fileName(seg.filePath)}
											</span>
											<button
												type="button"
												onClick={(e) => {
													e.stopPropagation();
													deleteSegment(index(), seg.id);
												}}
												class="p-1 rounded hover:bg-gray-3 text-gray-11 hover:text-red-400"
												aria-label="Delete overlay"
											>
												<IconLucideTrash2 class="size-4" />
											</button>
										</div>
										<div
											class="grid grid-cols-2 gap-x-3 gap-y-1"
											onClick={(e) => e.stopPropagation()}
										>
											<NumberField
												label="Start (s)"
												value={seg.start}
												step={0.1}
												onChange={(v) => updateSegment(seg.id, { start: v })}
											/>
											<NumberField
												label="End (s)"
												value={seg.end}
												step={0.1}
												onChange={(v) => updateSegment(seg.id, { end: v })}
											/>
											<NumberField
												label="Center X"
												value={seg.center.x}
												step={0.01}
												min={0}
												max={1}
												onChange={(v) =>
													updateSegment(seg.id, {
														center: { ...seg.center, x: v },
													})
												}
											/>
											<NumberField
												label="Center Y"
												value={seg.center.y}
												step={0.01}
												min={0}
												max={1}
												onChange={(v) =>
													updateSegment(seg.id, {
														center: { ...seg.center, y: v },
													})
												}
											/>
											<NumberField
												label="Size X"
												value={seg.size.x}
												step={0.01}
												min={0}
												max={1}
												onChange={(v) =>
													updateSegment(seg.id, { size: { ...seg.size, x: v } })
												}
											/>
											<NumberField
												label="Size Y"
												value={seg.size.y}
												step={0.01}
												min={0}
												max={1}
												onChange={(v) =>
													updateSegment(seg.id, { size: { ...seg.size, y: v } })
												}
											/>
											<NumberField
												label="Opacity"
												value={seg.opacity}
												step={0.05}
												min={0}
												max={1}
												onChange={(v) => updateSegment(seg.id, { opacity: v })}
											/>
											<NumberField
												label="Fade (s)"
												value={seg.fadeDuration}
												step={0.05}
												min={0}
												onChange={(v) =>
													updateSegment(seg.id, { fadeDuration: v })
												}
											/>
											<AnimField
												label="Anim in"
												value={seg.animIn ?? ""}
												onChange={(v) => updateSegment(seg.id, { animIn: v })}
											/>
											<AnimField
												label="Anim out"
												value={seg.animOut ?? ""}
												onChange={(v) => updateSegment(seg.id, { animOut: v })}
											/>
											<NumberField
												label="Anim (s)"
												value={seg.animDuration}
												step={0.05}
												min={0}
												onChange={(v) =>
													updateSegment(seg.id, { animDuration: v })
												}
											/>
										</div>
									</li>
								);
							}}
						</For>
					</ul>
				</Show>
			</div>
		</div>
	);
}

function NumberField(props: {
	label: string;
	value: number;
	step?: number;
	min?: number;
	max?: number;
	onChange: (value: number) => void;
}) {
	return (
		<label class="flex flex-col gap-0.5">
			<span class="text-[0.65rem] text-gray-11">{props.label}</span>
			<input
				type="number"
				value={props.value}
				step={props.step ?? 1}
				min={props.min}
				max={props.max}
				onChange={(e) => {
					const parsed = Number.parseFloat(e.currentTarget.value);
					if (!Number.isNaN(parsed)) props.onChange(parsed);
				}}
				class="w-full rounded border border-gray-3 bg-gray-1 px-2 py-1 text-xs text-gray-12"
			/>
		</label>
	);
}

function AnimField(props: {
	label: string;
	value: string;
	onChange: (value: OverlayAnim | null) => void;
}) {
	return (
		<label class="flex flex-col gap-0.5">
			<span class="text-[0.65rem] text-gray-11">{props.label}</span>
			<select
				value={props.value}
				onChange={(e) => {
					const v = e.currentTarget.value;
					props.onChange(v === "" ? null : (v as OverlayAnim));
				}}
				class="w-full rounded border border-gray-3 bg-gray-1 px-2 py-1 text-xs text-gray-12"
			>
				<For each={ANIM_OPTIONS}>
					{(opt) => <option value={opt.value}>{opt.label}</option>}
				</For>
			</select>
		</label>
	);
}
