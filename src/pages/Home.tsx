// The Play page: start the game, patch status, quick links into the game folder.

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FileText, FolderCog, FolderOpen, Image, Play, RefreshCw } from "lucide-react";
import { api, errorText, joinPath } from "@/lib/api";
import { keys, useGameCheck, useLauncherState, usePatchStatus } from "@/lib/queries";
import { useStore } from "@/lib/store";
import { Badge, Button, Card, Notice } from "@/components/ui";
import { Backdrop } from "@/components/Backdrop";

export function Home() {
  const state = useLauncherState();
  const folder = state.data?.gameFolder ?? "";
  const patch = usePatchStatus(folder);
  const check = useGameCheck(folder);
  const { run, setRun, failure, setFailure, setWizard } = useStore();
  const client = useQueryClient();
  const [error, setError] = useState("");
  const [updating, setUpdating] = useState(false);

  const play = async () => {
    setError("");
    setFailure(null);
    try {
      setRun("starting");
      await api.launchGame(folder);
      const onPlay = state.data?.onPlay ?? "minimize";
      if (onPlay === "minimize") await getCurrentWindow().minimize();
      if (onPlay === "close") await getCurrentWindow().close();
    } catch (e) {
      setRun("idle");
      setError(errorText(e));
    }
  };

  const update = async () => {
    setUpdating(true);
    setError("");
    try {
      await api.installPatch(folder);
      await client.invalidateQueries({ queryKey: keys.patch(folder) });
    } catch (e) {
      setError(errorText(e));
    } finally {
      setUpdating(false);
    }
  };

  const p = patch.data;
  const supported = check.data?.build === "supported";
  const installed = p && (p.state === "installed" || p.state === "outdated" || p.state === "byHand");
  const running = run !== "idle" || p?.running;
  const open = (path: string, create = false) => api.openFolder(path, create).catch((e) => setError(errorText(e)));

  return (
    <div className="relative min-h-full">
      <Backdrop />
      <div className="relative flex min-h-full flex-col px-10 pb-8 pt-16">
        <div className="max-w-xl">
          <div className="mb-3 flex flex-wrap gap-2">
            {p?.state === "installed" && <Badge tone="good">Patch {p.installedVersion}</Badge>}
            {p?.state === "outdated" && <Badge tone="warn">Update available: {p.installedVersion} → {p.bundledVersion}</Badge>}
            {p?.state === "byHand" && <Badge tone="warn">Installed by hand</Badge>}
            {p?.state === "notInstalled" && <Badge tone="bad">Not installed</Badge>}
            {check.data && !supported && <Badge tone="bad">Unsupported runblack.exe</Badge>}
          </div>
          <h1 className="font-display text-5xl leading-tight text-ivory-50">Black &amp; White</h1>
          <p className="mt-3 text-ivory-300">
            Lionhead's god game on current Windows, with the patch's fixes and its Direct3D&nbsp;9 renderer.
          </p>
          <div className="mt-8 flex items-center gap-3">
            <Button variant="primary" size="lg" icon={<Play className="size-5" />} onClick={play}
              disabled={!installed || !supported || !!running} busy={run === "starting"}>
              {running && run !== "starting" ? "Playing" : "Play"}
            </Button>
            {(p?.state === "outdated" || p?.state === "byHand" || p?.state === "notInstalled") && (
              <Button icon={<RefreshCw className="size-4" />} onClick={update} busy={updating}>
                {p.state === "notInstalled" ? "Install" : "Update"} to {p.bundledVersion}
              </Button>
            )}
          </div>
          <div className="mt-4 max-w-xl space-y-3">
            {error && <Notice kind="error">{error}</Notice>}
            {check.data && !supported && <Notice kind="warning">{check.data.message}</Notice>}
            {failure && (
              <Notice kind="error" title="The game did not start">
                BWLaunch.exe reported a problem (exit code {failure.launcherExitCode ?? "?"}).
                {failure.logTail && <pre className="mt-2 max-h-40 overflow-auto rounded bg-ink-950/70 p-2 font-mono text-[11px] leading-snug text-ivory-300">{failure.logTail}</pre>}
              </Notice>
            )}
          </div>
        </div>

        <div className="mt-auto grid gap-4 pt-12 lg:grid-cols-[2fr_1fr]">
          <Card title="Game folder" actions={<Button size="sm" variant="ghost" icon={<FolderCog className="size-4" />} onClick={() => setWizard(true)}>Change…</Button>}>
            <p className="break-all font-mono text-xs text-ivory-300">{folder}</p>
            <div className="mt-4 flex flex-wrap gap-2">
              <Button size="sm" icon={<FolderOpen className="size-4" />} onClick={() => open(folder)}>Open folder</Button>
              <Button size="sm" icon={<Image className="size-4" />} onClick={() => open(joinPath(folder, "Screenshots"), true)}>Screenshots</Button>
              <Button size="sm" icon={<FileText className="size-4" />} onClick={() => open(joinPath(folder, "BWDecomp"))}>Documentation</Button>
            </div>
          </Card>
          <Card title="Log">
            <p className="text-xs text-ivory-400">BWDecomp.log in the game folder says what happened. Include it when you report a problem.</p>
            <LogPeek folder={folder} />
          </Card>
        </div>
      </div>
    </div>
  );
}

function LogPeek({ folder }: { folder: string }) {
  const [text, setText] = useState<string | null>(null);
  return (
    <div className="mt-3">
      {text === null ? (
        <Button size="sm" onClick={() => api.readLogTail(folder, 60).then((t) => setText(t || "(empty)"))}>Show the end of the log</Button>
      ) : (
        <pre className="max-h-48 overflow-auto rounded-lg bg-ink-950 p-3 font-mono text-[11px] leading-snug text-ivory-300">{text}</pre>
      )}
    </div>
  );
}
