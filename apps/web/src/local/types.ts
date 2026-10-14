import type { LocalContext } from "./contracts";
export type LocalActions = {
  generation: string;
  context: LocalContext;
  online: boolean;
  reload: () => void;
  read: <T>(
    path: string,
    decode: (value: unknown) => T,
    signal?: AbortSignal,
  ) => Promise<T>;
  write: <T>(
    path: string,
    method: string,
    input: unknown,
    decode: (value: unknown) => T,
  ) => Promise<T>;
};
