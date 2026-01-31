export interface Env {
  BUCKET: R2Bucket;
  API_KEY: string;
}

export function authenticate(request: Request, env: Env): Response | null {
  const authHeader = request.headers.get("Authorization");

  if (!authHeader) {
    return new Response(JSON.stringify({ error: "Missing Authorization header" }), {
      status: 401,
      headers: { "Content-Type": "application/json" },
    });
  }

  const [scheme, token] = authHeader.split(" ");

  if (scheme !== "Bearer" || !token) {
    return new Response(JSON.stringify({ error: "Invalid Authorization format. Use: Bearer <token>" }), {
      status: 401,
      headers: { "Content-Type": "application/json" },
    });
  }

  if (token !== env.API_KEY) {
    return new Response(JSON.stringify({ error: "Invalid API key" }), {
      status: 403,
      headers: { "Content-Type": "application/json" },
    });
  }

  return null; // Auth successful
}

export function corsHeaders(origin?: string | null): HeadersInit {
  return {
    "Access-Control-Allow-Origin": origin || "*",
    "Access-Control-Allow-Methods": "GET, PUT, DELETE, PATCH, OPTIONS",
    "Access-Control-Allow-Headers": "Authorization, Content-Type",
    "Access-Control-Max-Age": "86400",
  };
}
