// Settings, generated from settings/schema.ts: edit, then Save (or Revert / Reset to defaults).

import { useEffect, useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderOpen, RotateCcw, Save, Undo2 } from "lucide-react";
import { api, errorText, joinPath } from "@/lib/api";
import { keys, useLauncherState } from "@/lib/queries";
import { FIELDS, TABS, type Tab, type Values } from "@/settings/schema";
import { hints, isEmptyChange, toChange, toValues } from "@/settings/logic";
import { ResolutionField, SettingField } from "@/components/SettingField";
import { Button, Card, Notice, cx } from "@/components/ui";

export function Settings() {
  const state = useLauncherState();
  const folder = state.data?.gameFolder ?? "";
  const client = useQueryClient();
  const snapshot = useQuery({ queryKey: keys.settings(folder), queryFn: () => api.readSettings(folder), enabled: !!folder });
  const original = useMemo(() => (snapshot.data ? toValues(snapshot.data) : null), [snapshot.data]);
  const [values, setValues] = useState<Values>({});
  const [tab, setTab] = useState<Tab>("game");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ kind: "success" | "error"; text: string } | null>(null);

  // A new snapshot (first load, or after Save) replaces the edits.
  useEffect(() => {
    // oxlint-disable-next-line react/set-state-in-effect
    if (original) setValues(original);
  }, [original]);

  const ready = !!original && Object.keys(values).length > 0;
  const change = ready ? toChange(original, values) : null;
  const dirty = !!change && !isEmptyChange(change);
  const h = hints(values);
  const onChange = (id: string, v: string) => {
    setMessage(null);
    setValues((old) => ({ ...old, [id]: v }));
  };

  const save = async () => {
    if (!change) return;
    setBusy(true);
    try {
      await api.writeSettings(folder, change);
      await client.invalidateQueries({ queryKey: keys.settings(folder) });
      setMessage({ kind: "success", text: "Saved. Changes apply the next time the game starts." });
    } catch (e) {
      setMessage({ kind: "error", text: errorText(e) });
    } finally {
      setBusy(false);
    }
  };

  // Defaults for BWDecomp.ini; the game's own registry values stay as they are.
  const reset = () => {
    if (!snapshot.data || !original) return;
    const defaults = toValues(snapshot.data, true);
    const next = { ...values };
    for (const f of FIELDS) if (f.store === "ini") next[f.id] = defaults[f.id];
    setValues(next);
    setMessage(null);
  };

  const maxDetail = snapshot.data?.game.maxdetailidx;
  return (
    <div className="mx-auto max-w-4xl px-8 py-8">
      <header className="mb-6 flex items-end justify-between gap-4">
        <div>
          <h1 className="font-display text-3xl text-ivory-50">Settings</h1>
          <p className="mt-1 text-sm text-ivory-400">
            Saved to BWDecomp.ini in the game folder and to the game's own settings in the registry.
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="ghost" icon={<RotateCcw className="size-4" />} onClick={reset} title="BWDecomp.ini as shipped">Defaults</Button>
          <Button variant="ghost" icon={<Undo2 className="size-4" />} onClick={() => original && setValues(original)} disabled={!dirty}>Revert</Button>
          <Button variant="primary" icon={<Save className="size-4" />} onClick={save} busy={busy} disabled={!dirty}>Save</Button>
        </div>
      </header>

      {snapshot.error && <Notice kind="error">{errorText(snapshot.error)}</Notice>}
      {message && <div className="mb-4"><Notice kind={message.kind}>{message.text}</Notice></div>}
      {snapshot.data && !snapshot.data.iniExists && (
        <div className="mb-4"><Notice kind="warning">The game folder has no BWDecomp.ini; saving creates one from the shipped defaults.</Notice></div>
      )}

      <div role="tablist" aria-label="Settings sections" className="mb-4 flex flex-wrap gap-1 border-b border-ink-700">
        {TABS.map((t) => (
          <button key={t.id} role="tab" aria-selected={tab === t.id} onClick={() => setTab(t.id)}
            className={cx(
              "-mb-px border-b-2 px-4 py-2 text-sm transition-colors",
              tab === t.id ? "border-gold-400 text-ivory-50" : "border-transparent text-ivory-400 hover:text-ivory-100",
            )}>
            {t.label}
          </button>
        ))}
      </div>

      {ready && (
        <Card>
          <p className="mb-2 text-sm text-ivory-400">{TABS.find((t) => t.id === tab)?.blurb}</p>
          {tab === "game" && <ResolutionField values={values} onChange={onChange} hint={h["game.ScreenW"]} />}
          {FIELDS.filter((f) => f.tab === tab).map((f) => (
            <SettingField key={f.id} field={f} values={values} onChange={onChange} hint={h[f.id]} maxDetail={maxDetail} />
          ))}
          {tab === "textures" && (
            <div className="flex gap-2 pt-4">
              <Button size="sm" icon={<FolderOpen className="size-4" />} onClick={() => api.openFolder(joinPath(folder, "Textures", "Replace"), true)}>Open Textures\Replace</Button>
              <Button size="sm" icon={<FolderOpen className="size-4" />} onClick={() => api.openFolder(joinPath(folder, "Textures", "Dump"), true)}>Open Textures\Dump</Button>
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
