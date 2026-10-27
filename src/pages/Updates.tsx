// Updates from GitHub releases, for the launcher and for the patch (two repositories), and the launcher's options.

import { useEffect, useState, type ReactNode } from "react";
import { useQueryClient, type UseQueryResult } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ask } from "@tauri-apps/plugin-dialog";
import { Download, ExternalLink, RefreshCw, Save } from "lucide-react";
import { api, errorText, type LauncherState, type UpdateInfo } from "@/lib/api";
import { keys, useAppInfo, useLauncherState, usePatchStatus, useSaveState, useUpdateCheck } from "@/lib/queries";
import { Badge, Button, Card, Notice, Segmented, Switch, inputClass } from "@/components/ui";
import { Row } from "@/components/SettingField";

export function Updates() {
  const info = useAppInfo();
  const state = useLauncherState();
  const saveState = useSaveState();
  const client = useQueryClient();
  const folder = state.data?.gameFolder ?? null;
  const patch = usePatchStatus(folder);
  const [draft, setDraft] = useState<LauncherState | null>(null);
  const [busy, setBusy] = useState<"launcher" | "patch" | null>(null);
  const [error, setError] = useState("");
  const [done, setDone] = useState("");

  // The same queries as App's check on start: results it already has show at once.
  const automatic = !!state.data?.checkUpdatesOnStart;
  const launcher = useUpdateCheck("launcher", automatic);
  const patchUpdate = useUpdateCheck("patch", automatic && !!folder);

  useEffect(() => {
    if (state.data) setDraft(state.data);
  }, [state.data]);

  const checking = launcher.isFetching || patchUpdate.isFetching;
  const checkNow = () => {
    setError("");
    setDone("");
    void launcher.refetch();
    if (folder) void patchUpdate.refetch();
  };

  const installLauncher = async (update: UpdateInfo) => {
    const ok = await ask(
      `Download launcher ${update.latestVersion}, check it against the release's SHA-256 sums and run its setup? The launcher closes while it updates.`,
      { title: "Update the launcher", kind: "info" },
    );
    if (!ok) return;
    setBusy("launcher");
    setError("");
    try {
      await api.installLauncherUpdate(update);
    } catch (e) {
      setError(errorText(e));
      setBusy(null);
    }
  };

  const installPatch = async (update: UpdateInfo) => {
    if (!folder) return;
    const ok = await ask(
      `Download patch ${update.latestVersion}, check it against the release's SHA-256 sums and install it into ${folder}? Your BWDecomp.ini is kept.`,
      { title: "Update the patch", kind: "info" },
    );
    if (!ok) return;
    setBusy("patch");
    setError("");
    setDone("");
    try {
      await api.installPatchUpdate(folder, update);
      setDone(`Patch ${update.latestVersion} is installed.`);
      await client.invalidateQueries({ queryKey: keys.patch(folder) });
      await client.invalidateQueries({ queryKey: ["update", "patch"] });
      await client.invalidateQueries({ queryKey: keys.addons(folder) });
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const save = () => draft && saveState.mutate(draft, { onError: (e) => setError(errorText(e)) });
  const dirty = JSON.stringify(draft) !== JSON.stringify(state.data);
  const installedPatch = patch.data?.installedVersion || "";

  return (
    <div className="mx-auto max-w-4xl space-y-6 px-8 py-8">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h1 className="font-display text-3xl text-ivory-50">Updates</h1>
          <p className="mt-1 text-sm text-ivory-400">
            The launcher and the patch are released separately. Each launcher carries the patch of its day; newer
            patches install from here without waiting for a new launcher.
          </p>
        </div>
        <Button icon={<RefreshCw className="size-4" />} onClick={checkNow} busy={checking}>Check now</Button>
      </header>

      {error && <Notice kind="error">{error}</Notice>}
      {done && <Notice kind="success">{done}</Notice>}

      <UpdateCard
        title="Launcher"
        repo={state.data?.launcherRepo ?? ""}
        query={launcher}
        versions={<Badge>Launcher {info.data?.launcherVersion}</Badge>}
        busy={busy === "launcher"}
        onInstall={installLauncher}
      />

      <UpdateCard
        title="Patch"
        repo={state.data?.patchRepo ?? ""}
        query={patchUpdate}
        versions={
          <>
            <Badge>Installed {installedPatch || "none"}</Badge>
            <Badge>In this launcher {info.data?.decompVersion || "none"}</Badge>
          </>
        }
        busy={busy === "patch"}
        onInstall={installPatch}
        disabledReason={!folder ? "Set up the game folder first." : patch.data?.running ? "Close Black & White first." : ""}
      />

      {draft && (
        <Card title="Launcher options" actions={<Button variant="primary" size="sm" icon={<Save className="size-4" />} disabled={!dirty} busy={saveState.isPending} onClick={save}>Save</Button>}>
          <Row label="Launcher releases" help="GitHub repository checked for new launchers, as owner/name. Leave it empty for the default.">
            <input className={`${inputClass} w-80`} value={draft.launcherRepo} placeholder="owner/name" spellCheck={false}
              onChange={(e) => setDraft({ ...draft, launcherRepo: e.target.value.trim() })} />
          </Row>
          <Row label="Patch releases" help="GitHub repository checked for new patches, as owner/name. Leave it empty for the default.">
            <input className={`${inputClass} w-80`} value={draft.patchRepo} placeholder="owner/name" spellCheck={false}
              onChange={(e) => setDraft({ ...draft, patchRepo: e.target.value.trim() })} />
          </Row>
          <Row label="Check for updates on start">
            <Switch label="Check for updates on start" checked={draft.checkUpdatesOnStart} onChange={(v) => setDraft({ ...draft, checkUpdatesOnStart: v })} />
          </Row>
          <Row label="When the game starts" help="What the launcher window does while you play.">
            <Segmented label="When the game starts" value={draft.onPlay}
              options={[{ value: "keep", label: "Stay open" }, { value: "minimize", label: "Minimise" }, { value: "close", label: "Close" }]}
              onChange={(v) => setDraft({ ...draft, onPlay: v as LauncherState["onPlay"] })} />
          </Row>
          <Row label="Game arguments" help="Extra command-line arguments passed to runblack.exe, separated by spaces.">
            <input className={`${inputClass} w-72 font-mono`} value={draft.gameArguments.join(" ")} spellCheck={false}
              onChange={(e) => setDraft({ ...draft, gameArguments: e.target.value.split(" ").filter(Boolean) })} />
          </Row>
        </Card>
      )}
    </div>
  );
}

function UpdateCard({ title, repo, query, versions, busy, onInstall, disabledReason = "" }: {
  title: string;
  repo: string;
  query: UseQueryResult<UpdateInfo>;
  versions: ReactNode;
  busy: boolean;
  onInstall: (update: UpdateInfo) => void;
  disabledReason?: string;
}) {
  const update = query.data;
  const complete = !!update?.download && !!update?.sums;
  return (
    <Card title={title}>
      <div className="flex flex-wrap items-center gap-2">
        {versions}
        {repo && (
          <button className="ml-auto text-xs text-ink-500 underline-offset-2 hover:text-ivory-300 hover:underline"
            onClick={() => openUrl(`https://github.com/${repo}/releases`)}>
            {repo}
          </button>
        )}
      </div>
      {query.isFetching && !update && <p className="mt-4 text-sm text-ivory-400">Checking…</p>}
      {query.error && !query.isFetching && <div className="mt-4"><Notice kind="warning">{errorText(query.error)}</Notice></div>}
      {!update && !query.error && !query.isFetching && (
        <p className="mt-4 text-sm text-ivory-400">Not checked yet. Use Check now.</p>
      )}
      {update && (
        <div className="mt-4 space-y-3">
          {update.newer ? (
            <Notice kind="success" title={`Version ${update.latestVersion} is available`}>{update.name}</Notice>
          ) : (
            <Notice title="Up to date">Latest release: {update.latestVersion}.</Notice>
          )}
          {update.newer && update.notes && (
            <pre className="max-h-56 overflow-auto whitespace-pre-wrap rounded-lg bg-ink-950 p-3 text-xs text-ivory-300">{update.notes.trim()}</pre>
          )}
          <div className="flex gap-2">
            {update.newer && (
              <Button variant="primary" icon={<Download className="size-4" />} onClick={() => onInstall(update)} busy={busy}
                disabled={!complete || !!disabledReason}>
                Download and install
              </Button>
            )}
            <Button variant="ghost" icon={<ExternalLink className="size-4" />} onClick={() => openUrl(update.pageUrl)}>Release page</Button>
          </div>
          {update.newer && disabledReason && <p className="text-xs text-ivory-400">{disabledReason}</p>}
          {update.newer && !complete && (
            <Notice kind="warning">
              This release has no {update.download ? "SHA-256 sums file" : "download"} for the launcher to check, so it
              is not installed from here. Get it from the release page instead.
            </Notice>
          )}
        </div>
      )}
    </Card>
  );
}
