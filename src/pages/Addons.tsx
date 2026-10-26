// Addons: install from a folder or zip, enable per game folder, remove. Kinds are listed in ADDON_KINDS (and must
// match KINDS in src-tauri/src/addons.rs).

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { FileArchive, FolderOpen, FolderPlus, Image, Puzzle, SlidersHorizontal, Trash2 } from "lucide-react";
import { api, errorText, type AddonInfo } from "@/lib/api";
import { keys, useAppInfo, useLauncherState } from "@/lib/queries";
import { Badge, Button, Card, Notice, Switch } from "@/components/ui";

export const ADDON_KINDS: Record<string, { label: string; icon: typeof Puzzle; unit: string }> = {
  "texture-pack": { label: "Texture pack", icon: Image, unit: "images" },
  preset: { label: "Settings preset", icon: SlidersHorizontal, unit: "settings" },
};

export function Addons() {
  const state = useLauncherState();
  const info = useAppInfo();
  const folder = state.data?.gameFolder ?? null;
  const client = useQueryClient();
  const addons = useQuery({ queryKey: keys.addons(folder), queryFn: () => api.listAddons(folder) });
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = () => client.invalidateQueries({ queryKey: keys.addons(folder) });
  const run = async (id: string, action: () => Promise<unknown>) => {
    setBusy(id);
    setError("");
    try {
      await action();
      await refresh();
      if (folder) await client.invalidateQueries({ queryKey: keys.settings(folder) });
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const importFrom = async (directory: boolean) => {
    const picked = await open(
      directory
        ? { directory: true, title: "Addon folder (with addon.json)" }
        : { title: "Addon zip", filters: [{ name: "Addon", extensions: ["zip"] }] },
    );
    if (typeof picked === "string") await run("import", () => api.importAddon(picked));
  };

  return (
    <div className="mx-auto max-w-4xl px-8 py-8">
      <header className="mb-6 flex items-end justify-between gap-4">
        <div>
          <h1 className="font-display text-3xl text-ivory-50">Addons</h1>
          <p className="mt-1 text-sm text-ivory-400">Texture packs and settings presets, switched on and off without touching files by hand.</p>
        </div>
        <div className="flex gap-2">
          <Button icon={<FileArchive className="size-4" />} onClick={() => importFrom(false)} busy={busy === "import"}>Add zip…</Button>
          <Button icon={<FolderPlus className="size-4" />} onClick={() => importFrom(true)}>Add folder…</Button>
        </div>
      </header>

      {error && <div className="mb-4"><Notice kind="error">{error}</Notice></div>}

      {addons.data?.length === 0 && (
        <Card>
          <div className="flex flex-col items-center py-8 text-center">
            <Puzzle className="size-10 text-ink-500" />
            <h2 className="mt-4 font-display text-xl text-ivory-50">No addons yet</h2>
            <p className="mt-2 max-w-md text-sm text-ivory-400">
              An addon is a folder with an <code className="text-ivory-200">addon.json</code>, or a zip of one. Texture packs
              keep their images in a <code>textures</code> folder; presets list settings to apply.
            </p>
          </div>
        </Card>
      )}

      <div className="space-y-3">
        {addons.data?.map((a) => (
          <AddonRow key={a.manifest.id} addon={a} folder={folder} busy={busy === a.manifest.id}
            onToggle={(on) => folder && run(a.manifest.id, () => api.setAddonEnabled(folder, a.manifest.id, on))}
            onDelete={() => run(a.manifest.id, () => api.deleteAddon(a.manifest.id))} />
        ))}
      </div>

      <p className="mt-8 text-xs text-ink-500">
        Addons are kept in{" "}
        <button className="underline hover:text-ivory-300" onClick={() => info.data && api.openFolder(info.data.addonsDir, true)}>
          {info.data?.addonsDir}
        </button>
        . Texture packs go to Textures\Replace\&lt;id&gt; in the game folder and need the Direct3D 9 renderer.
      </p>
    </div>
  );
}

function AddonRow({ addon, folder, busy, onToggle, onDelete }: {
  addon: AddonInfo; folder: string | null; busy: boolean; onToggle: (on: boolean) => void; onDelete: () => void;
}) {
  const m = addon.manifest;
  const kind = ADDON_KINDS[m.kind];
  const Icon = kind?.icon ?? Puzzle;
  return (
    <Card>
      <div className="flex items-start gap-4">
        <div className="grid size-11 shrink-0 place-items-center rounded-lg bg-ink-800"><Icon className="size-5 text-gold-400" /></div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-medium text-ivory-50">{m.name}</h3>
            <span className="text-xs text-ivory-400">{m.version}</span>
            <Badge>{kind?.label ?? m.kind}</Badge>
            {addon.enabled && <Badge tone="good">Enabled</Badge>}
          </div>
          {m.author && <p className="text-xs text-ivory-400">by {m.author}</p>}
          {m.description && <p className="mt-2 text-sm text-ivory-300">{m.description}</p>}
          <p className="mt-1 text-xs text-ink-500">{addon.fileCount} {kind?.unit ?? "items"}{m.requires.decomp ? ` · needs decompilation ${m.requires.decomp}` : ""}</p>
          {addon.problem && <div className="mt-2"><Notice kind="warning">{addon.problem}</Notice></div>}
        </div>
        <div className="flex items-center gap-1">
          <Button size="sm" variant="ghost" title="Open folder" aria-label="Open addon folder" onClick={() => api.openFolder(addon.path)}><FolderOpen className="size-4" /></Button>
          <Button size="sm" variant="ghost" title="Remove" aria-label="Remove addon" onClick={onDelete} disabled={addon.enabled || busy}><Trash2 className="size-4" /></Button>
          <div className="ml-2"><Switch label={`Enable ${m.name}`} checked={addon.enabled} onChange={onToggle} disabled={busy || !folder || (!!addon.problem && !addon.enabled)} /></div>
        </div>
      </div>
    </Card>
  );
}
