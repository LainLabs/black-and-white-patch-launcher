// Shared TanStack Query hooks: one cache key per backend read, invalidated after writes.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type LauncherState } from "./api";

export const keys = {
  info: ["info"] as const,
  state: ["state"] as const,
  patch: (folder: string) => ["patch", folder] as const,
  check: (folder: string) => ["check", folder] as const,
  settings: (folder: string) => ["settings", folder] as const,
  addons: (folder: string | null) => ["addons", folder] as const,
  modes: ["modes"] as const,
};

export const useAppInfo = () => useQuery({ queryKey: keys.info, queryFn: api.appInfo, staleTime: Infinity });
export const useLauncherState = () => useQuery({ queryKey: keys.state, queryFn: api.getState });

export function useSaveState() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (value: LauncherState) => api.setState(value),
    onSuccess: () => client.invalidateQueries({ queryKey: keys.state }),
  });
}

export const usePatchStatus = (folder: string | null | undefined) =>
  useQuery({
    queryKey: keys.patch(folder ?? ""),
    queryFn: () => api.patchStatus(folder!),
    enabled: !!folder,
  });

export const useGameCheck = (folder: string | null | undefined) =>
  useQuery({
    queryKey: keys.check(folder ?? ""),
    queryFn: () => api.checkGame(folder!),
    enabled: !!folder,
    retry: false,
  });

export const useDisplayModes = () => useQuery({ queryKey: keys.modes, queryFn: api.displayModes, staleTime: Infinity });
