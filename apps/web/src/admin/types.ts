import type { AdminConfiguration } from "@/generated/AdminConfiguration";
import type { UserInfo } from "@/generated/UserInfo";
import type { GroupInfo } from "@/generated/GroupInfo";
export interface AdminData {
  config: AdminConfiguration;
  users: UserInfo[];
  groups: GroupInfo[];
}
export interface Actions {
  execute: (path: string, method: string, body: unknown) => Promise<boolean>;
  reload: () => void;
  busy: boolean;
  error: unknown;
}
