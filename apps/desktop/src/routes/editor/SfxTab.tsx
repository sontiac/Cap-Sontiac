import { Button } from "@cap/ui-solid";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appLocalDataDir, join } from "@tauri-apps/api/path";
import { exists, mkdir, readDir } from "@tauri-apps/plugin-fs";
import * as shell from "@tauri-apps/plugin-shell";
import { createResource, For, Show } from "solid-js";
import { commands } from "~/utils/tauri";
import IconLucideFolderOpen from "~icons/lucide/folder-open";
import IconLucideRefreshCw from "~icons/lucide/refresh-cw";
import { serializeProjectConfiguration, useEditorContext } from "./context";

const SFX_FOLDER = "sound_effects";
const SFX_DEFAULT_DURATION = 1;
const SFX_EXTENSIONS = /\.(mp3|wav|m4a|ogg|aac|flac)$/i;

interface SfxFile {
	name: string;
	path: string;
}

interface SfxSegmentShape {
	id: string;
	start: number;
	end: number;
	filePath: string;
	volume: number;
}

async function ensureSfxDir(): Promise<string> {
	const dir = await join(await appLocalDataDir(), SFX_FOLDER);
	if (!(await exists(dir))) {
		await mkdir(dir, { recursive: true });
	}
	return dir;
}

async function listSfxFiles(): Promise<SfxFile[]> {
	const dir = await ensureSfxDir();
	const entries = await readDir(dir);
	return entries
		.filter((e) => e.isFile && SFX_EXTENSIONS.test(e.name))
		.map((e) => ({ name: e.name, path: `${dir}/${e.name}` }));
}

function previewSfx(file: SfxFile) {
	const audio = new Audio(convertFileSrc(file.path));
	audio.play().catch((err) => console.error("Failed to preview SFX:", err));
}

export function SfxTab() {
	const { project, setProject, editorState } = useEditorContext();
	const [files, { refetch }] = createResource(listSfxFiles, {
		initialValue: [],
	});

	const insertAtPlayhead = async (file: SfxFile) => {
		if (!project.timeline) return;
		const time = editorState.previewTime ?? editorState.playbackTime;
		const newSegment: SfxSegmentShape = {
			id: crypto.randomUUID(),
			start: time,
			end: time + SFX_DEFAULT_DURATION,
			filePath: file.path,
			volume: 1.0,
		};
		const timeline = project.timeline as {
			sfxSegments?: SfxSegmentShape[];
		} & typeof project.timeline;
		const existing = timeline.sfxSegments ?? [];
		const updated = {
			...project,
			timeline: {
				...project.timeline,
				sfxSegments: [...existing, newSegment],
			},
		} as typeof project;
		setProject(updated);
		await commands.setProjectConfig(serializeProjectConfiguration(updated));
	};

	const openSfxFolder = async () => {
		const dir = await ensureSfxDir();
		await shell.open(dir);
	};

	return (
		<div class="flex flex-col gap-3">
			<div class="flex items-center justify-between">
				<h3 class="text-sm font-medium text-gray-12">Sound effects</h3>
				<div class="flex gap-1">
					<button
						type="button"
						onClick={() => refetch()}
						class="p-1 rounded hover:bg-gray-3 text-gray-11 hover:text-gray-12"
						aria-label="Refresh"
					>
						<IconLucideRefreshCw class="size-4" />
					</button>
					<button
						type="button"
						onClick={openSfxFolder}
						class="p-1 rounded hover:bg-gray-3 text-gray-11 hover:text-gray-12"
						aria-label="Open SFX folder"
					>
						<IconLucideFolderOpen class="size-4" />
					</button>
				</div>
			</div>
			<Show
				when={(files() ?? []).length > 0}
				fallback={
					<p class="text-xs text-gray-11">
						Drop audio files (mp3, wav, m4a, ogg, aac, flac) into your
						sound_effects folder, then click refresh.
					</p>
				}
			>
				<ul class="flex flex-col gap-1">
					<For each={files() ?? []}>
						{(file) => (
							<li class="flex items-center gap-2 rounded p-2 hover:bg-gray-3">
								<button
									type="button"
									onClick={() => previewSfx(file)}
									class="flex-1 text-left text-xs truncate text-gray-12"
									title={file.name}
								>
									{file.name}
								</button>
								<Button
									variant="primary"
									size="sm"
									onClick={() => insertAtPlayhead(file)}
								>
									Insert
								</Button>
							</li>
						)}
					</For>
				</ul>
			</Show>
		</div>
	);
}
