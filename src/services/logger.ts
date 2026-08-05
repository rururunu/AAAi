export type LogLevel = "debug" | "info" | "warn" | "error";

export type LogEntry = {
  ts: number;
  level: LogLevel;
  scope: string;
  message: string;
  detail?: unknown;
};

const LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 10,
  info: 20,
  warn: 30,
  error: 40,
};

const RING_CAPACITY = 200;
const ring: LogEntry[] = [];

function minLevel(): LogLevel {
  return import.meta.env.DEV ? "debug" : "warn";
}

function shouldLog(level: LogLevel): boolean {
  return LEVEL_ORDER[level] >= LEVEL_ORDER[minLevel()];
}

function pushRing(entry: LogEntry): void {
  ring.push(entry);
  if (ring.length > RING_CAPACITY) {
    ring.splice(0, ring.length - RING_CAPACITY);
  }
}

function formatPrefix(scope: string, level: LogLevel): string {
  const time = new Date().toISOString().slice(11, 23);
  return `[${time}] [${level.toUpperCase()}] [${scope}]`;
}

function write(level: LogLevel, scope: string, message: string, detail?: unknown): void {
  const entry: LogEntry = {
    ts: Date.now(),
    level,
    scope,
    message,
    detail,
  };
  pushRing(entry);
  if (!shouldLog(level)) return;

  const prefix = formatPrefix(scope, level);
  const args = detail === undefined ? [prefix, message] : [prefix, message, detail];
  switch (level) {
    case "debug":
      console.debug(...args);
      break;
    case "info":
      console.info(...args);
      break;
    case "warn":
      console.warn(...args);
      break;
    case "error":
      console.error(...args);
      break;
  }
}

export type Logger = {
  debug: (message: string, detail?: unknown) => void;
  info: (message: string, detail?: unknown) => void;
  warn: (message: string, detail?: unknown) => void;
  error: (message: string, detail?: unknown) => void;
};

export function createLogger(scope: string): Logger {
  return {
    debug: (message, detail) => write("debug", scope, message, detail),
    info: (message, detail) => write("info", scope, message, detail),
    warn: (message, detail) => write("warn", scope, message, detail),
    error: (message, detail) => write("error", scope, message, detail),
  };
}

/** Recent log entries for debug panels (newest last). */
export function getRecentLogs(limit = RING_CAPACITY): LogEntry[] {
  return ring.slice(-limit);
}

export function clearRecentLogs(): void {
  ring.length = 0;
}

export const rootLogger = createLogger("app");
