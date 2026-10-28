// Top level: the first-run wizard until a game folder is set up, then the launcher with its side navigation.

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Download, Home as HomeIcon, Info, Puzzle, SlidersHorizontal } from "lucide-react";
import { api } from "@/lib/api";
import { keys, useAppInfo, useLauncherState, useUpdateCheck } from "@/lib/queries";
import { useStore, type Page } from "@/lib/store";
import { Mark, cx } from "@/components/ui";
import { Wizard } from "@/pages/Wizard";
import { Home } from "@/pages/Home";
import { Settings } from "@/pages/Settings";
import { Addons } from "@/pages/Addons";
import { Updates } from "@/pages/Updates";
import { About } from "@/pages/About";

const NAV: { id: Page; label: string; icon: typeof HomeIcon }[] = [
  { id: "home", label: "Play", icon: HomeIcon },
  { id: "settings", label: "Settings", icon: SlidersHorizontal },
  { id: "addons", label: "Addons", icon: Puzzle },
  { id: "updates", label: "Updates", icon: Download },
  { id: "about", label: "About", icon: Info },
];

export function App() {
  const state = useLauncherState();
  const info = useAppInfo();
  const { page, setPage, wizard, setRun, setFailure } = useStore();
  const client = useQueryClient();

  // Game events from the backend: started / failed / exited.
  useEffect(() => {
    const unlisten = api.onGameEvent((event) => {
      const folder = state.data?.gameFolder;
      if (event.state === "started") {
        setRun("running");
        setFailure(null);
      } else {
        setRun("idle");
        setFailure(event.state === "failed" ? event : null);
        void getCurrentWindow().unminimize();
      }
      if (folder) void client.invalidateQueries({ queryKey: keys.patch(folder) });
    });
    return () => void unlisten.then((f) => f());
  }, [client, setFailure, setRun, state.data?.gameFolder]);

  // Quiet update checks on start, launcher and patch (the Updates page shows details); failures are ignored here.
  const automatic = !!state.data?.checkUpdatesOnStart;
  const launcherUpdate = useUpdateCheck("launcher", automatic);
  const patchUpdate = useUpdateCheck("patch", automatic && !!state.data?.gameFolder);
  const updateAvailable = !!launcherUpdate.data?.newer || !!patchUpdate.data?.newer;

  if (state.isLoading || info.isLoading) {
    return (
      <div className="grid h-full place-items-center">
        <Mark className="size-16 animate-pulse" />
      </div>
    );
  }
  if (!state.data?.gameFolder || wizard) {
    return <Wizard />;
  }

  return (
    <div className="flex h-full">
      <nav className="flex w-56 shrink-0 flex-col border-r border-ink-800 bg-ink-900/90 px-3 py-5" aria-label="Main">
        <div className="mb-8 flex items-center gap-3 px-2">
          <Mark className="size-9" />
          <div className="leading-tight">
            <div className="font-display text-base text-ivory-50">Black &amp; White</div>
            <div className="text-xs text-ivory-400">Launcher</div>
          </div>
        </div>
        <ul className="space-y-1">
          {NAV.map(({ id, label, icon: Icon }) => (
            <li key={id}>
              <button
                onClick={() => setPage(id)}
                aria-current={page === id ? "page" : undefined}
                className={cx(
                  "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors",
                  page === id ? "bg-ink-700 text-ivory-50" : "text-ivory-400 hover:bg-ink-800 hover:text-ivory-100",
                )}
              >
                <Icon className={cx("size-4", page === id && "text-gold-400")} />
                {label}
                {id === "updates" && updateAvailable && (
                  <span className="ml-auto size-2 rounded-full bg-gold-400" aria-label="Update available" />
                )}
              </button>
            </li>
          ))}
        </ul>
        <div className="mt-auto px-2 text-[11px] leading-relaxed text-ink-500">
          Launcher {info.data?.launcherVersion}
          <br />
          Patch {info.data?.decompVersion || "not bundled"}
        </div>
      </nav>
      <main className="min-w-0 flex-1 overflow-y-auto">
        {page === "home" && <Home />}
        {page === "settings" && <Settings />}
        {page === "addons" && <Addons />}
        {page === "updates" && <Updates />}
        {page === "about" && <About />}
      </main>
    </div>
  );
}
