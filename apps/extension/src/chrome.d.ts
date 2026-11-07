// The few extension APIs Fitifact uses, typed here instead of pulling in @types/chrome.
declare namespace chrome {
  namespace runtime {
    function getURL(path: string): string;
  }
  namespace storage {
    interface StorageChange {
      newValue?: unknown;
      oldValue?: unknown;
    }
    const local: {
      get(key: string): Promise<Record<string, unknown>>;
      set(items: Record<string, unknown>): Promise<void>;
    };
    const onChanged: {
      addListener(listener: (changes: Record<string, StorageChange>, area: string) => void): void;
    };
  }
  namespace tabs {
    function create(options: { url: string }): Promise<unknown>;
  }
}
