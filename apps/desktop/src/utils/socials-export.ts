import type { AspectRatio, Video, XY } from "~/utils/tauri";

// Mirrors ProjectUniforms::get_base_size + get_output_size in
// crates/rendering/src/lib.rs: passing the base size as resolution_base makes
// the renderer's fit scale exactly 1.0, so the export is never letterboxed,
// pillarboxed, or downscaled. Keep in sync with the Rust side.
const SCREEN_MAX_PADDING = 0.4;

const ASPECT_RATIO_TARGETS: Record<AspectRatio, number> = {
	square: 1,
	wide: 16 / 9,
	vertical: 9 / 16,
	classic: 4 / 3,
	tall: 3 / 4,
};

export type SocialsCanvasConfig = {
	aspectRatio: AspectRatio | null;
	background: { crop: { size: XY<number> } | null; padding: number };
};

function roundBaseDimension(value: number): number {
	return Math.max((Math.ceil(value) + 1) & ~1, 2);
}

export function nativeCanvasSize(
	config: SocialsCanvasConfig,
	display: Pick<Video, "width" | "height">,
): XY<number> {
	const cropSize = config.background.crop?.size ?? {
		x: display.width,
		y: display.height,
	};
	const paddingFactor = (config.background.padding / 100) * SCREEN_MAX_PADDING;

	if (!config.aspectRatio) {
		const scale = 1 + paddingFactor * 2;
		return {
			x: (Math.trunc(cropSize.x * scale) + 1) & ~1,
			y: (Math.trunc(cropSize.y * scale) + 1) & ~1,
		};
	}

	const target = ASPECT_RATIO_TARGETS[config.aspectRatio];
	const padding = Math.max(cropSize.x, cropSize.y) * paddingFactor * 2;
	if (cropSize.x / cropSize.y > target) {
		const width = cropSize.x + padding;
		return {
			x: roundBaseDimension(width),
			y: roundBaseDimension(width / target),
		};
	}
	const height = cropSize.y + padding;
	return {
		x: roundBaseDimension(height * target),
		y: roundBaseDimension(height),
	};
}

export function renderedOutputSize(base: XY<number>): XY<number> {
	return { x: (base.x + 3) & ~3, y: (base.y + 1) & ~1 };
}
