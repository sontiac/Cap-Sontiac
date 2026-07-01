import { createEventListenerMap } from "@solid-primitives/event-listener";
import { cx } from "cva";
import { createMemo, createRoot, Show } from "solid-js";
import { produce } from "solid-js/store";

import type { OverlaySegment } from "~/utils/tauri";
import { useEditorContext } from "./context";

type OverlaySegmentOverlayProps = {
	size: { width: number; height: number };
};

const MIN_SIZE = 0.02;
const MAX_CENTER = 1;

export function OverlaySegmentOverlay(props: OverlaySegmentOverlayProps) {
	const { project, setProject, editorState, setEditorState, projectHistory } =
		useEditorContext();

	const currentAbsoluteTime = () =>
		editorState.previewTime ?? editorState.playbackTime ?? 0;

	const selectedIndex = createMemo(() => {
		const selection = editorState.timeline.selection;
		if (!selection || selection.type !== "overlay") return null;
		return selection.indices[0] ?? null;
	});

	const selectedSegment = createMemo(() => {
		const index = selectedIndex();
		if (index === null) return null;
		const segment = project.timeline?.overlaySegments?.[index];
		if (!segment) return null;
		return { index, segment };
	});

	const shouldShowHandles = createMemo(() => {
		const sel = selectedSegment();
		if (!sel) return false;
		const time = currentAbsoluteTime();
		return time >= sel.segment.start && time < sel.segment.end;
	});

	const updateSelectedSegment = (fn: (segment: OverlaySegment) => void) => {
		const index = selectedIndex();
		if (index === null) return;
		setProject("timeline", "overlaySegments", index, produce(fn));
	};

	const handleBackgroundClick = (e: MouseEvent) => {
		if (e.target === e.currentTarget && selectedIndex() !== null) {
			e.preventDefault();
			e.stopPropagation();
			setEditorState("timeline", "selection", null);
		}
	};

	return (
		<div class="absolute inset-0 pointer-events-none">
			<Show when={selectedIndex() !== null}>
				<div
					class="absolute inset-0 pointer-events-auto"
					onMouseDown={handleBackgroundClick}
				/>
			</Show>
			<Show when={shouldShowHandles() ? selectedSegment() : null}>
				{(sel) => (
					<OverlayHandles
						segment={sel().segment}
						size={props.size}
						updateSegment={updateSelectedSegment}
						projectHistory={projectHistory}
					/>
				)}
			</Show>
		</div>
	);
}

function OverlayHandles(props: {
	segment: OverlaySegment;
	size: { width: number; height: number };
	updateSegment: (fn: (segment: OverlaySegment) => void) => void;
	projectHistory: ReturnType<typeof useEditorContext>["projectHistory"];
}) {
	function createMouseDownDrag<T>(
		setup: () => T,
		update: (
			e: MouseEvent,
			value: T,
			initialMouse: { x: number; y: number },
		) => void,
	) {
		return (downEvent: MouseEvent) => {
			downEvent.preventDefault();
			downEvent.stopPropagation();

			const initial = setup();
			const initialMouse = { x: downEvent.clientX, y: downEvent.clientY };
			const resumeHistory = props.projectHistory.pause();

			function handleUpdate(event: MouseEvent) {
				update(event, initial, initialMouse);
			}

			function finish() {
				resumeHistory();
				dispose();
			}

			const dispose = createRoot((dispose) => {
				createEventListenerMap(window, {
					mousemove: handleUpdate,
					mouseup: () => {
						finish();
					},
				});
				return dispose;
			});
		};
	}

	const center = () => props.segment.center ?? { x: 0.5, y: 0.5 };
	const size = () => props.segment.size ?? { x: 0.5, y: 0.3 };

	const rect = () => {
		const width = size().x * props.size.width;
		const height = size().y * props.size.height;
		const left = center().x * props.size.width - width / 2;
		const top = center().y * props.size.height - height / 2;
		return { width, height, left, top };
	};

	const onMove = createMouseDownDrag(
		() => ({
			startCenter: { ...center() },
		}),
		(e, { startCenter }, initialMouse) => {
			const dx = (e.clientX - initialMouse.x) / props.size.width;
			const dy = (e.clientY - initialMouse.y) / props.size.height;

			props.updateSegment((s) => {
				s.center = {
					x: Math.max(0, Math.min(MAX_CENTER, startCenter.x + dx)),
					y: Math.max(0, Math.min(MAX_CENTER, startCenter.y + dy)),
				};
			});
		},
	);

	const createResizeHandler = (dirX: -1 | 0 | 1, dirY: -1 | 0 | 1) => {
		return createMouseDownDrag(
			() => ({
				startCenter: { ...center() },
				startSize: { ...size() },
			}),
			(e, { startCenter, startSize }, initialMouse) => {
				const dx = (e.clientX - initialMouse.x) / props.size.width;
				const dy = (e.clientY - initialMouse.y) / props.size.height;

				props.updateSegment((s) => {
					const nextCenter = { ...(s.center ?? startCenter) };
					const nextSize = { ...(s.size ?? startSize) };

					if (dirX !== 0) {
						nextSize.x = Math.max(MIN_SIZE, startSize.x + dx * dirX);
						nextCenter.x = startCenter.x + dx / 2;
					}

					if (dirY !== 0) {
						nextSize.y = Math.max(MIN_SIZE, startSize.y + dy * dirY);
						nextCenter.y = startCenter.y + dy / 2;
					}

					s.center = nextCenter;
					s.size = nextSize;
				});
			},
		);
	};

	return (
		<div
			class="absolute pointer-events-auto group z-10"
			style={{
				left: `${rect().left}px`,
				top: `${rect().top}px`,
				width: `${rect().width}px`,
				height: `${rect().height}px`,
			}}
			onMouseDown={onMove}
		>
			<div class="absolute inset-0 rounded-md border-2 border-gray-12 bg-gray-9/10 cursor-move" />

			<ResizeHandle
				class="top-0 left-0 -translate-x-1/2 -translate-y-1/2 cursor-nw-resize"
				onMouseDown={createResizeHandler(-1, -1)}
			/>
			<ResizeHandle
				class="top-0 right-0 translate-x-1/2 -translate-y-1/2 cursor-ne-resize"
				onMouseDown={createResizeHandler(1, -1)}
			/>
			<ResizeHandle
				class="bottom-0 left-0 -translate-x-1/2 translate-y-1/2 cursor-sw-resize"
				onMouseDown={createResizeHandler(-1, 1)}
			/>
			<ResizeHandle
				class="bottom-0 right-0 translate-x-1/2 translate-y-1/2 cursor-se-resize"
				onMouseDown={createResizeHandler(1, 1)}
			/>
			<ResizeHandle
				class="top-0 left-1/2 -translate-x-1/2 -translate-y-1/2 cursor-n-resize"
				onMouseDown={createResizeHandler(0, -1)}
			/>
			<ResizeHandle
				class="bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 cursor-s-resize"
				onMouseDown={createResizeHandler(0, 1)}
			/>
			<ResizeHandle
				class="left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 cursor-w-resize"
				onMouseDown={createResizeHandler(-1, 0)}
			/>
			<ResizeHandle
				class="right-0 top-1/2 translate-x-1/2 -translate-y-1/2 cursor-e-resize"
				onMouseDown={createResizeHandler(1, 0)}
			/>
		</div>
	);
}

function ResizeHandle(props: {
	class?: string;
	onMouseDown: (e: MouseEvent) => void;
}) {
	return (
		<div
			class={cx(
				"absolute w-3 h-3 bg-gray-12 border border-white rounded-full shadow-xs transition-transform hover:scale-125",
				props.class,
			)}
			onMouseDown={props.onMouseDown}
		/>
	);
}
