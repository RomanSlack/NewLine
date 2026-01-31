import { authenticate, corsHeaders, type Env } from "./auth";
import { listFiles, getFile, putFile, deleteFile } from "./handlers/files";
import { getState, updateState } from "./handlers/state";

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    const path = url.pathname;
    const method = request.method;
    const origin = request.headers.get("Origin");

    // Handle CORS preflight
    if (method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: corsHeaders(origin),
      });
    }

    // Authenticate all other requests
    const authError = authenticate(request, env);
    if (authError) {
      return addCorsHeaders(authError, origin);
    }

    try {
      let response: Response;

      // Route handling
      if (path === "/api/v1/files" && method === "GET") {
        // List all files
        response = await listFiles(env);
      } else if (path.startsWith("/api/v1/files/")) {
        // Single file operations
        const filePath = path.substring("/api/v1/files/".length);

        if (!filePath) {
          response = new Response(JSON.stringify({ error: "File path required" }), {
            status: 400,
            headers: { "Content-Type": "application/json" },
          });
        } else if (method === "GET") {
          response = await getFile(env, filePath);
        } else if (method === "PUT") {
          response = await putFile(env, filePath, request);
        } else if (method === "DELETE") {
          response = await deleteFile(env, filePath);
        } else {
          response = new Response(JSON.stringify({ error: "Method not allowed" }), {
            status: 405,
            headers: { "Content-Type": "application/json" },
          });
        }
      } else if (path === "/api/v1/state") {
        // State operations
        if (method === "GET") {
          response = await getState(env);
        } else if (method === "PATCH") {
          response = await updateState(env, request);
        } else {
          response = new Response(JSON.stringify({ error: "Method not allowed" }), {
            status: 405,
            headers: { "Content-Type": "application/json" },
          });
        }
      } else if (path === "/api/v1/health" || path === "/") {
        // Health check endpoint
        response = new Response(JSON.stringify({ status: "ok", version: "1.0.0" }), {
          headers: { "Content-Type": "application/json" },
        });
      } else {
        response = new Response(JSON.stringify({ error: "Not found" }), {
          status: 404,
          headers: { "Content-Type": "application/json" },
        });
      }

      return addCorsHeaders(response, origin);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Internal server error";
      return addCorsHeaders(
        new Response(JSON.stringify({ error: message }), {
          status: 500,
          headers: { "Content-Type": "application/json" },
        }),
        origin
      );
    }
  },
};

function addCorsHeaders(response: Response, origin: string | null): Response {
  const newHeaders = new Headers(response.headers);
  const cors = corsHeaders(origin);

  for (const [key, value] of Object.entries(cors)) {
    newHeaders.set(key, value);
  }

  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers: newHeaders,
  });
}
