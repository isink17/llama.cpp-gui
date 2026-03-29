export type RemoteApiConfig = {
  baseUrl: string;
  token: string;
};

export type RemoteEventEnvelope<T = unknown> = {
  kind: string;
  payload: T;
};

const STORAGE_KEY = 'llamacppdesk.remoteApiConfig';
const ENV_BASE_URL = import.meta.env.VITE_REMOTE_API_BASE_URL as string | undefined;
const ENV_TOKEN = import.meta.env.VITE_REMOTE_API_TOKEN as string | undefined;

const normalizeBaseUrl = (baseUrl: string) => baseUrl.trim().replace(/\/+$/, '');

const safeStorage = () => {
  if (typeof window === 'undefined') {
    return null;
  }

  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

export const getRemoteApiConfig = (): RemoteApiConfig | null => {
  const storage = safeStorage();
  if (!storage) {
    const baseUrl = typeof ENV_BASE_URL === 'string' ? normalizeBaseUrl(ENV_BASE_URL) : '';
    const token = typeof ENV_TOKEN === 'string' ? ENV_TOKEN.trim() : '';
    return baseUrl && token ? { baseUrl, token } : null;
  }

  const raw = storage.getItem(STORAGE_KEY);
  if (!raw) {
    return null;
  }

  try {
    const parsed = JSON.parse(raw) as Partial<RemoteApiConfig>;
    const baseUrl = typeof parsed.baseUrl === 'string' ? normalizeBaseUrl(parsed.baseUrl) : '';
    const token = typeof parsed.token === 'string' ? parsed.token.trim() : '';

    if (!baseUrl || !token) {
      const envBaseUrl = typeof ENV_BASE_URL === 'string' ? normalizeBaseUrl(ENV_BASE_URL) : '';
      const envToken = typeof ENV_TOKEN === 'string' ? ENV_TOKEN.trim() : '';
      return envBaseUrl && envToken ? { baseUrl: envBaseUrl, token: envToken } : null;
    }

    return { baseUrl, token };
  } catch {
    return null;
  }
};

export const getRemoteApiDefaults = (): RemoteApiConfig | null => {
  const baseUrl = typeof ENV_BASE_URL === 'string' ? normalizeBaseUrl(ENV_BASE_URL) : '';
  const token = typeof ENV_TOKEN === 'string' ? ENV_TOKEN.trim() : '';
  return baseUrl && token ? { baseUrl, token } : null;
};

export const setRemoteApiConfig = (config: RemoteApiConfig) => {
  const storage = safeStorage();
  if (!storage) {
    return;
  }

  storage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      baseUrl: normalizeBaseUrl(config.baseUrl),
      token: config.token.trim(),
    }),
  );
};

export const clearRemoteApiConfig = () => {
  const storage = safeStorage();
  if (!storage) {
    return;
  }

  storage.removeItem(STORAGE_KEY);
};

export const hasRemoteApiConfig = () => getRemoteApiConfig() !== null;

const parseResponseBody = async <T>(response: Response): Promise<T> => {
  if (response.status === 204) {
    return undefined as T;
  }

  const text = await response.text();
  if (!text) {
    return undefined as T;
  }

  try {
    return JSON.parse(text) as T;
  } catch {
    return text as T;
  }
};

const extractErrorMessage = async (response: Response) => {
  const text = await response.text();
  if (!text) {
    return `${response.status} ${response.statusText}`.trim();
  }

  try {
    const parsed = JSON.parse(text) as { error?: string };
    if (typeof parsed.error === 'string' && parsed.error.trim()) {
      return parsed.error.trim();
    }
  } catch {
    // Fall through to raw text.
  }

  return text.trim();
};

export const remoteRequest = async <T>(
  path: string,
  init: RequestInit = {},
): Promise<T> => {
  const config = getRemoteApiConfig();
  if (!config) {
    throw new Error('Remote API is not configured. Set the browser mode base URL and token first.');
  }

  const response = await fetch(`${config.baseUrl}${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${config.token}`,
      'Content-Type': 'application/json',
      ...(init.headers ?? {}),
    },
  });

  if (!response.ok) {
    throw new Error(await extractErrorMessage(response));
  }

  return parseResponseBody<T>(response);
};

export const subscribeToRemoteChatEvents = async (
  handler: (event: RemoteEventEnvelope) => void,
): Promise<() => void> => {
  const config = getRemoteApiConfig();
  if (!config) {
    return () => {};
  }

  const controller = new AbortController();

  void (async () => {
    const response = await fetch(`${config.baseUrl}/api/events`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${config.token}`,
        Accept: 'text/event-stream',
      },
      signal: controller.signal,
    });

    if (!response.ok || !response.body) {
      throw new Error(`Failed to open remote event stream (${response.status})`);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    const current = { event: 'message', data: [] as string[] };
    let buffer = '';

    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() ?? '';

      for (const line of lines) {
        if (line.length === 0) {
          if (current.data.length > 0 && current.event === 'chat') {
            try {
              handler(JSON.parse(current.data.join('\n')) as RemoteEventEnvelope);
            } catch {
              // Ignore malformed stream events.
            }
          }
          current.event = 'message';
          current.data = [];
          continue;
        }

        if (line.startsWith('event:')) {
          current.event = line.slice('event:'.length).trim();
          continue;
        }

        if (line.startsWith('data:')) {
          current.data.push(line.slice('data:'.length).trimStart());
        }
      }
    }

    if (current.data.length > 0 && current.event === 'chat') {
      try {
        handler(JSON.parse(current.data.join('\n')) as RemoteEventEnvelope);
      } catch {
        // Ignore malformed stream events.
      }
    }
  })().catch(() => {
    // Stream failures are handled by the caller's polling fallback.
  });

  return () => controller.abort();
};
