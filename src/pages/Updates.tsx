// Updates from GitHub releases, and the launcher's own options.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ask } from "@tauri-apps/plugin-dialog";
import { Download, ExternalLink, RefreshCw, Save } from "lucide-react";
import { api, errorText, type LauncherState, type UpdateInfo } from "@/lib/api";
import { useAppInfo, useLauncherState, useSaveState } from "@/lib/queries";
import { Badge, Button, Card, Notice, Segmented, Switch, inputClass } from "@/components/ui";
import { Row } from "@/components/SettingField";

export function Updates() {
  const info = useAppInfo();
  const state = useLauncherState();
  const saveState = useSaveState();
  const [draft, setDraft] = useState<LauncherState | null>(null);
  const client = useQueryClient();
  // The quiet check App runs on start, if it found something.
  const [update, setUpdate] = useState<UpdateInfo | null>(
    () => client.getQueryData<UpdateInfo>(["update", state.data?.updateRepo ?? ""]) ?? null,
  );
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (state.data) setDraft(state.data);
  }, [state.data]);

  const check = async () => {
    if (!draft?.updateRepo) return;
    setChecking(true);
    setError("");
    try {
      setUpdate(await api.checkUpdates(draft.updateRepo));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setChecking(false);
    }
  };

  const install = async () => {
    if (!update) return;
    const ok = await ask(`Download version ${update.latestVersion}, check it against the release's SHA-256 sums and run its setup? The launcher closes while it updates.`, { title: "Update the launcher", kind: "info" });
    if (!ok) return;
    setInstalling(true);
    setError("");
    try {
      await api.installUpdate(update);
    } catch (e) {
      setError(errorText(e));
      setInstalling(false);
    }
  };

  const save = () => draft && saveState.mutate(draft, { onError: (e) => setError(errorText(e)) });
  const dirty = JSON.stringify(draft) !== JSON.stringify(state.data);

  return (
    <div className="mx-auto max-w-4xl space-y-6 px-8 py-8">
      <header>
        <h1 className="font-display text-3xl text-ivory-50">Updates</h1>
        <p className="mt-1 text-sm text-ivory-400">
          Each launcher release carries a build of the decompilation, so updating the launcher updates the game's patch too.
        </p>
      </header>

      {error && <Notice kind="error">{error}</Notice>}

      <Card title="Versions" actions={
        <Button icon={<RefreshCw className="size-4" />} onClick={check} busy={checking} disabled={!draft?.updateRepo}>Check now</Button>
      }>
        <div className="flex flex-wrap gap-2">
          <Badge>Launcher {info.data?.launcherVersion}</Badge>
          <Badge>Decompilation {info.data?.decompVersion || "not bundled"}</Badge>
        </div>
        {!draft?.updateRepo && <div className="mt-4"><Notice>No update repository: save to go back to the launcher's own.</Notice></div>}
        {update && (
          <div className="mt-4 space-y-3">
            {update.newer ? (
              <Notice kind="success" title={`Version ${update.latestVersion} is available`}>{update.name}</Notice>
            ) : (
              <Notice title="You have the latest version">Latest release: {update.latestVersion}.</Notice>
            )}
            {update.notes && (
              <pre className="max-h-56 overflow-auto whitespace-pre-wrap rounded-lg bg-ink-950 p-3 text-xs text-ivory-300">{update.notes}</pre>
            )}
            <div className="flex gap-2">
              {update.newer && (
                <Button variant="primary" icon={<Download className="size-4" />} onClick={install} busy={installing}
                  disabled={!update.installer || !update.sums}>
                  Download and install
                </Button>
              )}
              <Button variant="ghost" icon={<ExternalLink className="size-4" />} onClick={() => openUrl(update.pageUrl)}>Release page</Button>
            </div>
            {update.newer && (!update.installer || !update.sums) && (
              <Notice kind="warning">This release has no launcher setup program with SHA-256 sums; download it from the release page.</Notice>
            )}
          </div>
        )}
      </Card>

      {draft && (
        <Card title="Launcher options" actions={<Button variant="primary" size="sm" icon={<Save className="size-4" />} disabled={!dirty} busy={saveState.isPending} onClick={save}>Save</Button>}>
          <Row label="Update repository" help="GitHub repository whose releases are checked, as owner/name. Leave it empty for the launcher's own.">
            <input className={`${inputClass} w-72`} value={draft.updateRepo} placeholder="owner/name" spellCheck={false}
              onChange={(e) => setDraft({ ...draft, updateRepo: e.target.value.trim() })} />
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
