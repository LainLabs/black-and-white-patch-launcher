// Versions, documentation installed with the patch, removing the patch, legal note.

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { ask } from "@tauri-apps/plugin-dialog";
import { FileText, Trash2 } from "lucide-react";
import { api, errorText, joinPath } from "@/lib/api";
import { keys, useAppInfo, useLauncherState } from "@/lib/queries";
import { useStore } from "@/lib/store";
import { Badge, Button, Card, Mark, Notice } from "@/components/ui";

const DOCS = [
  { file: "README.txt", what: "Installing, playing, settings, problems" },
  { file: "FIXES.md", what: "Every fix, its cause and its setting" },
  { file: "RENDERER.md", what: "The Direct3D 9 renderer" },
  { file: "TEXTURES.md", what: "Making texture packs" },
  { file: "CHANGELOG.md", what: "What changed in each version" },
];

export function About() {
  const info = useAppInfo();
  const state = useLauncherState();
  const folder = state.data?.gameFolder ?? "";
  const client = useQueryClient();
  const setWizard = useStore((s) => s.setWizard);
  const [message, setMessage] = useState<{ kind: "success" | "error"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const remove = async () => {
    const ok = await ask(
      "Remove the patch from the game folder? Addons are disabled first. A BWDecomp.ini you changed, saved games, screenshots and texture packs stay.",
      { title: "Remove the patch", kind: "warning" },
    );
    if (!ok) return;
    setBusy(true);
    try {
      const result = await api.removePatch(folder);
      await client.invalidateQueries({ queryKey: keys.patch(folder) });
      setMessage({ kind: "success", text: result.kept.length ? `Removed. Kept because you changed them: ${result.kept.join(", ")}.` : "Removed." });
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mx-auto max-w-4xl space-y-6 px-8 py-8">
      <header className="flex items-center gap-4">
        <Mark className="size-14" />
        <div>
          <h1 className="font-display text-3xl text-ivory-50">About</h1>
          <div className="mt-1 flex flex-wrap gap-2">
            <Badge>Launcher {info.data?.launcherVersion}</Badge>
            <Badge>Patch {info.data?.decompVersion || "not bundled"}{info.data?.decompCommit ? ` (${info.data.decompCommit})` : ""}</Badge>
          </div>
        </div>
      </header>

      <Card title="Documentation">
        <p className="mb-3 text-sm text-ivory-400">Installed with the patch in the game's BWDecomp folder.</p>
        <ul className="divide-y divide-ink-800">
          {DOCS.map((d) => (
            <li key={d.file} className="flex items-center justify-between py-2.5">
              <div>
                <div className="text-sm text-ivory-50">{d.file}</div>
                <div className="text-xs text-ivory-400">{d.what}</div>
              </div>
              <FileText className="size-4 text-ink-500" />
            </li>
          ))}
        </ul>
        <Button className="mt-3" size="sm" onClick={() => api.openFolder(joinPath(folder, "BWDecomp")).catch((e) => setMessage({ kind: "error", text: errorText(e) }))}>
          Open the documentation folder
        </Button>
      </Card>

      <Card title="Game folder">
        <p className="break-all font-mono text-xs text-ivory-300">{folder}</p>
        {message && <div className="mt-3"><Notice kind={message.kind}>{message.text}</Notice></div>}
        <div className="mt-4 flex gap-2">
          <Button onClick={() => setWizard(true)}>Set up again…</Button>
          <Button variant="danger" icon={<Trash2 className="size-4" />} onClick={remove} busy={busy}>Remove the patch</Button>
        </div>
      </Card>

      <p className="text-xs leading-relaxed text-ink-500">
        Black &amp; White is a game by Lionhead Studios. This launcher and the patch are fan projects, not
        affiliated with or endorsed by Lionhead Studios or the game's publishers, and contain none of the game's code
        or data. You need your own copy of the game. Launcher state: {info.data?.stateDir}
      </p>
    </div>
  );
}
