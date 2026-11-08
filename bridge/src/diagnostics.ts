export class DiagnosticsBuffer {
  private readonly entries: string[] = [];

  constructor(private readonly limit = 50) {}

  push(line: string): void {
    const trimmed = line.trim();
    if (trimmed.length === 0) return;
    this.entries.push(trimmed.slice(0, 500));
    if (this.entries.length > this.limit) {
      this.entries.splice(0, this.entries.length - this.limit);
    }
  }

  list(): string[] {
    return [...this.entries];
  }
}

type ConsoleLike = {
  warn: (...args: unknown[]) => void;
  error: (...args: unknown[]) => void;
};

export function captureConsoleDiagnostics(
  buffer: DiagnosticsBuffer,
  target: ConsoleLike = console,
): () => void {
  const originalWarn = target.warn;
  const originalError = target.error;
  const format = (args: unknown[]): string =>
    args
      .map((arg) => (typeof arg === 'string' ? arg : (JSON.stringify(arg) ?? String(arg))))
      .join(' ');
  target.warn = (...args: unknown[]) => {
    buffer.push(format(args));
    originalWarn(...args);
  };
  target.error = (...args: unknown[]) => {
    buffer.push(format(args));
    originalError(...args);
  };
  return () => {
    target.warn = originalWarn;
    target.error = originalError;
  };
}
