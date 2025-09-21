declare global {
  interface Window {
    ippan?: {
      connect?: () => Promise<{ address: string; balance: string }>;
      connectHardware?: () => Promise<{ address: string; balance: string }>;
      signMessage?: (message: string) => Promise<string>;
      getAddress?: () => Promise<string>;
    };
  }
}

export {};
