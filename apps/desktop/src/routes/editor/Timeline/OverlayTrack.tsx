import { createEventListenerMap } from "@solid-primitives/event-listener";
import { convertFileSrc } from "@tauri-apps/api/core";
import { cx } from "cva";
import { createMemo, createRoot, Index, Show } from "solid-js";

import { useEditorContext } from "../context";
import { overlayThumbPath } from "../timeline-utils";
import { useTimelineContext, useTrackContext } from "./context";
import {
	SegmentContent,
	SegmentHandle,
	SegmentRoot,
	TrackRoot,
	useSetPreviewTime,
} from "./Track";

export type OverlaySegmentDragState =
	| { type: "idle" }
	| { type: "movePending" }
	| { type: "moving" };

const MIN_OVERLAY_SEGMENT_PIXEL_WIDTH = 40;

function fileName(path: string): string {
	return path.split(/[\\/]/).pop() ?? path;
}

export function OverlayTrack(props: {
	onDragStateChanged: (v: OverlaySegmentDragState) => void;
	handleUpdatePlayhead: (e: MouseEvent) => void;
}) {
	const {
		project,
		setProject,
		projectHistory,
		setEditorState,
		editorState,
		totalDuration,
	} = useEditorContext();

	const { secsPerPixel } = useTimelineContext();
	const setPreviewTime = useSetPreviewTime();

	const hasOverlaySegments = () =>
		(project.timeline?.overlaySegments?.length ?? 0) > 0;

	const selectedOverlayIndices = createMemo(() => {
		const selection = editorState.timeline.selection;
		if (!selection || selection.type !== "overlay") return null;
		return new Set(selection.indices);
	});

	return (
		<TrackRoot
			onMouseEnter={() => setEditorState("timeline", "hoveredTrack", "overlay")}
			onMouseLeave={() => setEditorState("timeline", "hoveredTrack", null)}
		>
			<Show
				when={hasOverlaySegments()}
				fallback={
					<div class="relative z-1 isolate text-center text-xs text-(--text-tertiary) flex flex-col gap-2 justify-center items-center inset-0 w-full bg-gray-3/10 dark:bg-gray-3/10 rounded-xl pointer-events-none px-2 py-1">
						<span>Add overlays from the Overlays tab</span>
					</div>
				}
			>
				<Index each={project.timeline?.overlaySegments}>
					{(segment, i) => {
						const { setTrackState } = useTrackContext();

						function createMouseDownDrag<T>(
							setup: () => T,
							_update: (e: MouseEvent, v: T, initialMouseX: number) => void,
						) {
							return (downEvent: MouseEvent) => {
								if (editorState.timeline.interactMode !== "seek") return;

								downEvent.stopPropagation();

								const initial = setup();

								let moved = false;
								let initialMouseX: null | number = null;

								setTrackState("draggingSegment", true);

								const resumeHistory = projectHistory.pause();

								props.onDragStateChanged({ type: "movePending" });

								function finish(e: MouseEvent) {
									resumeHistory();
									if (!moved) {
										e.stopPropagation();

										setEditorState("timeline", "selection", {
											type: "overlay",
											indices: [i],
										});
										props.handleUpdatePlayhead(e);
									}
									props.onDragStateChanged({ type: "idle" });
									setTrackState("draggingSegment", false);
								}

								function update(event: MouseEvent) {
									if (Math.abs(event.clientX - downEvent.clientX) > 2) {
										if (!moved) {
											moved = true;
											initialMouseX = event.clientX;
											props.onDragStateChanged({ type: "moving" });
										}
									}

									if (initialMouseX === null) return;

									_update(event, initial, initialMouseX);
								}

								createRoot((dispose) => {
									createEventListenerMap(window, {
										mousemove: (e) => {
											update(e);
										},
										mouseup: (e) => {
											update(e);
											finish(e);
											dispose();
										},
									});
								});
							};
						}

						const isSelected = createMemo(() => {
							const indices = selectedOverlayIndices();
							if (!indices) return false;
							return indices.has(i);
						});

						return (
							<SegmentRoot
								class={cx(
									"border duration-200 hover:border-gray-12 transition-colors group",
									"bg-linear-to-r from-[#1e3a5f] via-[#2e5a9a] to-[#1e3a5f] shadow-[inset_0_8px_12px_3px_rgba(255,255,255,0.15)]",
									isSelected() ? "border-gray-12" : "border-transparent",
								)}
								innerClass="ring-blue-5"
								segment={segment()}
							>
								<SegmentHandle
									position="start"
									onMouseDown={createMouseDownDrag(
										() => {
											const start = segment().start;
											const minDuration = Math.max(
												0.25,
												secsPerPixel() * MIN_OVERLAY_SEGMENT_PIXEL_WIDTH,
											);
											const maxValue = segment().end - minDuration;
											return { start, minValue: 0, maxValue };
										},
										(e, value, initialMouseX) => {
											const newStart =
												value.start +
												(e.clientX - initialMouseX) * secsPerPixel();
											const nextStart = Math.min(
												value.maxValue,
												Math.max(value.minValue, newStart),
											);

											setProject(
												"timeline",
												"overlaySegments",
												i,
												"start",
												nextStart,
											);
											setPreviewTime(nextStart);
										},
									)}
								/>
								<SegmentContent
									class="flex items-center gap-2 cursor-grab overflow-hidden"
									onMouseDown={createMouseDownDrag(
										() => {
											const original = { ...segment() };
											return { original };
										},
										(e, value, initialMouseX) => {
											const rawDelta =
												(e.clientX - initialMouseX) * secsPerPixel();
											const width = value.original.end - value.original.start;
											let newStart = value.original.start + rawDelta;
											if (newStart < 0) newStart = 0;
											if (newStart + width > totalDuration())
												newStart = Math.max(0, totalDuration() - width);

											setProject("timeline", "overlaySegments", i, {
												start: newStart,
												end: newStart + width,
											});
										},
									)}
								>
									<img
										src={convertFileSrc(overlayThumbPath(segment()))}
										alt=""
										class="size-8 object-contain shrink-0 pointer-events-none"
									/>
									<span class="text-xs truncate text-gray-1 dark:text-gray-12 pointer-events-none">
										{fileName(segment().filePath)}
									</span>
								</SegmentContent>
								<SegmentHandle
									position="end"
									onMouseDown={createMouseDownDrag(
										() => {
											const end = segment().end;
											const minDuration = Math.max(
												0.25,
												secsPerPixel() * MIN_OVERLAY_SEGMENT_PIXEL_WIDTH,
											);
											const minValue = segment().start + minDuration;
											const maxValue = totalDuration();
											return { end, minValue, maxValue };
										},
										(e, value, initialMouseX) => {
											const newEnd =
												value.end +
												(e.clientX - initialMouseX) * secsPerPixel();
											const nextEnd = Math.min(
												value.maxValue,
												Math.max(value.minValue, newEnd),
											);

											setProject(
												"timeline",
												"overlaySegments",
												i,
												"end",
												nextEnd,
											);
											setPreviewTime(nextEnd);
										},
									)}
								/>
							</SegmentRoot>
						);
					}}
				</Index>
			</Show>
		</TrackRoot>
	);
}
