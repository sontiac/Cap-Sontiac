import {
	provideOptionalAuth,
	Storage,
	Videos,
	VideosRepo,
	verifyStorageObjectToken,
} from "@cap/web-backend";
import { Video } from "@cap/web-domain";
import { Effect, Option } from "effect";
import type { NextRequest } from "next/server";
import { runPromise } from "@/lib/server";
import { CACHE_CONTROL_HEADERS } from "@/utils/helpers";

export const dynamic = "force-dynamic";

const copyHeader = (
	source: Headers,
	target: Headers,
	sourceName: string,
	targetName = sourceName,
) => {
	const value = source.get(sourceName);
	if (value) target.set(targetName, value);
};

const getTokenVideo = (videoId: Video.VideoId) =>
	Effect.gen(function* () {
		const repo = yield* VideosRepo;
		const maybeVideo = yield* repo.getById(videoId);
		if (Option.isNone(maybeVideo)) return yield* Effect.fail("not-found");
		return maybeVideo.value[0];
	});

const getPolicyVideo = (videoId: Video.VideoId) =>
	Effect.gen(function* () {
		const videos = yield* Videos;
		const maybeVideo = yield* videos.getByIdForViewing(videoId).pipe(
			Effect.flatten,
			Effect.catchTag("NoSuchElementException", () =>
				Effect.fail("not-found" as const),
			),
		);
		return maybeVideo[0];
	});

export async function GET(request: NextRequest) {
	const videoIdParam = request.nextUrl.searchParams.get("videoId");
	const key = request.nextUrl.searchParams.get("key");
	const token = request.nextUrl.searchParams.get("token");

	if (!videoIdParam || !key) {
		return new Response("Missing videoId or key", { status: 400 });
	}

	const effect = Effect.gen(function* () {
		const tokenPayload = token ? verifyStorageObjectToken(token) : null;
		const videoId = Video.VideoId.make(videoIdParam);
		const video =
			tokenPayload?.videoId === videoIdParam && tokenPayload.key === key
				? yield* getTokenVideo(videoId)
				: yield* getPolicyVideo(videoId);

		if (!key.startsWith(`${video.ownerId}/${video.id}/`)) {
			return yield* Effect.fail("not-found" as const);
		}

		const [storage] = yield* Storage.getAccessForVideo(video);
		if (!("getObjectResponse" in storage)) {
			const url = yield* storage.getSignedObjectUrl(key);
			return Response.redirect(url);
		}

		const upstream = yield* storage.getObjectResponse(
			key,
			request.headers.get("range"),
		);
		const headers = new Headers(CACHE_CONTROL_HEADERS);
		copyHeader(upstream.headers, headers, "content-type", "Content-Type");
		copyHeader(upstream.headers, headers, "content-length", "Content-Length");
		copyHeader(upstream.headers, headers, "content-range", "Content-Range");
		copyHeader(upstream.headers, headers, "accept-ranges", "Accept-Ranges");
		if (!headers.has("Accept-Ranges")) headers.set("Accept-Ranges", "bytes");

		return new Response(upstream.body, {
			status: upstream.status,
			headers,
		});
	}).pipe(provideOptionalAuth);

	try {
		return await runPromise(effect);
	} catch {
		return new Response("Not found", { status: 404 });
	}
}

export const HEAD = GET;
