import type { Env } from "../auth";

export interface SyncState {
  pinned: string[];
  lastSync: string;
  version: number;
}

const STATE_KEY = "state.json";
const DEFAULT_STATE: SyncState = {
  pinned: [],
  lastSync: new Date(0).toISOString(),
  version: 1,
};

export async function getState(env: Env): Promise<Response> {
  const object = await env.BUCKET.get(STATE_KEY);

  if (!object) {
    return new Response(JSON.stringify(DEFAULT_STATE), {
      headers: { "Content-Type": "application/json" },
    });
  }

  const state = await object.json<SyncState>();

  return new Response(JSON.stringify(state), {
    headers: {
      "Content-Type": "application/json",
      "ETag": object.etag,
    },
  });
}

export async function updateState(env: Env, request: Request): Promise<Response> {
  // Get current state
  const object = await env.BUCKET.get(STATE_KEY);
  let currentState = DEFAULT_STATE;

  if (object) {
    currentState = await object.json<SyncState>();
  }

  // Parse patch
  const patch = await request.json<Partial<SyncState>>();

  // Merge state (shallow merge)
  const newState: SyncState = {
    ...currentState,
    ...patch,
    lastSync: new Date().toISOString(),
  };

  // Save updated state
  await env.BUCKET.put(STATE_KEY, JSON.stringify(newState), {
    httpMetadata: { contentType: "application/json" },
  });

  return new Response(JSON.stringify(newState), {
    headers: { "Content-Type": "application/json" },
  });
}
