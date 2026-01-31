import type { Env } from "../auth";

export interface FileMeta {
  path: string;
  size: number;
  etag: string;
  modified: string;
  contentType: string;
}

function sanitizePath(path: string): string {
  // Remove leading slashes and prevent directory traversal
  return path.replace(/^\/+/, "").replace(/\.\./g, "");
}

export async function listFiles(env: Env): Promise<Response> {
  const files: FileMeta[] = [];

  // List all files in the files/ prefix
  const listed = await env.BUCKET.list({ prefix: "files/" });

  for (const object of listed.objects) {
    // Skip directory markers
    if (object.key.endsWith("/")) continue;

    // Remove "files/" prefix for the path
    const path = object.key.substring(6);

    files.push({
      path,
      size: object.size,
      etag: object.etag,
      modified: object.uploaded.toISOString(),
      contentType: object.httpMetadata?.contentType || "text/plain",
    });
  }

  return new Response(JSON.stringify({ files }), {
    headers: { "Content-Type": "application/json" },
  });
}

export async function getFile(env: Env, path: string): Promise<Response> {
  const sanitized = sanitizePath(path);
  const key = `files/${sanitized}`;

  const object = await env.BUCKET.get(key);

  if (!object) {
    return new Response(JSON.stringify({ error: "File not found" }), {
      status: 404,
      headers: { "Content-Type": "application/json" },
    });
  }

  const headers = new Headers();
  headers.set("Content-Type", object.httpMetadata?.contentType || "text/plain; charset=utf-8");
  headers.set("ETag", object.etag);
  headers.set("X-Modified", object.uploaded.toISOString());

  return new Response(object.body, { headers });
}

export async function putFile(env: Env, path: string, request: Request): Promise<Response> {
  const sanitized = sanitizePath(path);
  const key = `files/${sanitized}`;

  const content = await request.text();
  const contentType = request.headers.get("Content-Type") || "text/plain; charset=utf-8";

  const object = await env.BUCKET.put(key, content, {
    httpMetadata: { contentType },
  });

  const meta: FileMeta = {
    path: sanitized,
    size: content.length,
    etag: object.etag,
    modified: new Date().toISOString(),
    contentType,
  };

  return new Response(JSON.stringify(meta), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}

export async function deleteFile(env: Env, path: string): Promise<Response> {
  const sanitized = sanitizePath(path);
  const key = `files/${sanitized}`;

  // Check if file exists first
  const object = await env.BUCKET.head(key);
  if (!object) {
    return new Response(JSON.stringify({ error: "File not found" }), {
      status: 404,
      headers: { "Content-Type": "application/json" },
    });
  }

  await env.BUCKET.delete(key);

  return new Response(JSON.stringify({ deleted: sanitized }), {
    headers: { "Content-Type": "application/json" },
  });
}
