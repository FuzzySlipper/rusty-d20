export interface ClockPort {
  readonly now: () => Date;
  readonly setTimeout: (callback: () => void, delayMs: number) => number;
  readonly clearTimeout: (handle: number) => void;
}

export interface KeyValueStoragePort {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

export interface ClipboardPort {
  readonly writeText: (value: string) => Promise<void>;
}

export interface DocumentEffectsPort {
  readonly setTitle: (title: string) => void;
  readonly setRootClass: (className: string, enabled: boolean) => void;
}

export interface HttpResponse {
  readonly status: number;
  readonly body: unknown;
}

export interface HttpPort {
  readonly getJson: (path: string) => Promise<HttpResponse>;
}

export const browserClock: ClockPort = {
  now: () => new Date(),
  setTimeout: (callback, delayMs) => window.setTimeout(callback, delayMs),
  clearTimeout: (handle) => window.clearTimeout(handle),
};

export const memoryStorage = (initial: Readonly<Record<string, string>> = {}): KeyValueStoragePort => {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
  };
};

export const browserStorage = (): KeyValueStoragePort => ({
  getItem: (key) => window.localStorage.getItem(key),
  setItem: (key, value) => window.localStorage.setItem(key, value),
  removeItem: (key) => window.localStorage.removeItem(key),
});

export const browserClipboard = (): ClipboardPort => ({
  writeText: (value) => navigator.clipboard.writeText(value),
});

export const browserDocumentEffects = (): DocumentEffectsPort => ({
  setTitle: (title) => {
    document.title = title;
  },
  setRootClass: (className, enabled) => document.documentElement.classList.toggle(className, enabled),
});

export const browserHttp = (): HttpPort => ({
  getJson: async (path) => {
    const response = await fetch(path, {
      headers: { accept: 'application/json' },
      method: 'GET',
    });
    const contentType = response.headers.get('content-type') ?? '';
    const body: unknown = contentType.includes('application/json') ? await response.json() : await response.text();
    return { status: response.status, body };
  },
});
