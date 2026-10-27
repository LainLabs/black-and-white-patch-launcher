// Shared TanStack Query hooks: one cache key per backend read, invalidated after writes.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type LauncherState, type Product } from "./api";

export const keys = {
  info: ["info"] as const,
  state: ["state"] as const,
  patch: (folder: string) => ["patch", folder] as const,
  check: (folder: string) => ["check", folder] as const,
  settings: (folder: string) => ["settings", folder] as const,
  addons: (folder: string | null) => ["addons", folder] as const,
  modes: ["modes"] as const,
  update: (product: Product, repo: string, folder: string | null) => ["update", product, repo, folder ?? ""] as const,
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

/**
 * The newest launcher or patch release, from the repository in the launcher's state. Runs by itself only when
 * `automatic` (the "check on start" option); `refetch()` checks on demand. Shared by App (the dot on Updates) and the
 * Updates page, so one check serves both.
 */
export function useUpdateCheck(product: Product, automatic: boolean) {
  const state = useLauncherState();
  const repo = (product === "launcher" ? state.data?.launcherRepo : state.data?.patchRepo) ?? "";
  const folder = state.data?.gameFolder ?? null;
  return useQuery({
    queryKey: keys.update(product, repo, folder),
    queryFn: () => api.checkUpdates(product, repo, folder),
    enabled: automatic && !!repo,
    retry: false,
    staleTime: 6 * 60 * 60 * 1000,
  });
}

export const useDisplayModes = () => useQuery({ queryKey: keys.modes, queryFn: api.displayModes, staleTime: Infinity });
