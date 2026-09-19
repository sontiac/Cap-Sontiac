import { describe, expect, it } from "vitest";
import { nativeCanvasSize, renderedOutputSize } from "./socials-export";

const display = { width: 1080, height: 2320 };

const config = (
	aspectRatio: Parameters<typeof nativeCanvasSize>[0]["aspectRatio"],
	padding = 0,
	crop: { size: { x: number; y: number } } | null = null,
) => ({ aspectRatio, background: { crop, padding } });

describe("nativeCanvasSize", () => {
	it("untouched project uses raw source size", () => {
		expect(nativeCanvasSize(config(null), display)).toEqual({
			x: 1080,
			y: 2320,
		});
	});

	it("vertical canvas expands a taller-than-9:16 source sideways", () => {
		expect(nativeCanvasSize(config("vertical"), display)).toEqual({
			x: 1306,
			y: 2320,
		});
	});

	it("wide canvas expands a portrait source to 16:9", () => {
		expect(nativeCanvasSize(config("wide"), display)).toEqual({
			x: 4126,
			y: 2320,
		});
	});

	it("crop already at the target aspect passes through even-rounded", () => {
		expect(
			nativeCanvasSize(
				config("vertical", 0, { size: { x: 900, y: 1600 } }),
				display,
			),
		).toEqual({ x: 900, y: 1600 });
	});

	it("padding scales the free-aspect canvas", () => {
		expect(
			nativeCanvasSize(
				config(null, 100, { size: { x: 1000, y: 1000 } }),
				display,
			),
		).toEqual({ x: 1800, y: 1800 });
	});

	it("padding expands a fixed-aspect canvas from the longest crop edge", () => {
		expect(
			nativeCanvasSize(
				config("square", 50, { size: { x: 1000, y: 2000 } }),
				display,
			),
		).toEqual({ x: 2800, y: 2800 });
	});

	it("crop wider than the target expands along width", () => {
		expect(
			nativeCanvasSize(
				config("square", 50, { size: { x: 2000, y: 1000 } }),
				display,
			),
		).toEqual({ x: 2800, y: 2800 });
	});

	it("truncates then even-rounds fractional free-aspect sizes", () => {
		expect(nativeCanvasSize(config(null, 10), display)).toEqual({
			x: 1166,
			y: 2506,
		});
	});
});

describe("renderedOutputSize", () => {
	it("rounds width up to a multiple of four and height up to even", () => {
		expect(renderedOutputSize({ x: 1306, y: 2320 })).toEqual({
			x: 1308,
			y: 2320,
		});
		expect(renderedOutputSize({ x: 1080, y: 2321 })).toEqual({
			x: 1080,
			y: 2322,
		});
	});
});
