// First run (and "Change game folder"): find the game, check runblack.exe, install the patch, pick the basics.

import { useEffect, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowLeft, ArrowRight, Check, FolderOpen, Play, Search, ShieldAlert } from "lucide-react";
import { api, errorText, type SettingsSnapshot } from "@/lib/api";
import { keys, useAppInfo, useGameCheck, useLauncherState, usePatchStatus } from "@/lib/queries";
import { useStore } from "@/lib/store";
import { FIELDS, type Values } from "@/settings/schema";
import { hints, toChange, toValues } from "@/settings/logic";
import { ResolutionField, SettingField } from "@/components/SettingField";
import { Badge, Button, Card, Mark, Notice, cx, inputClass } from "@/components/ui";
import { Backdrop } from "@/components/Backdrop";

const STEPS = ["Welcome", "Game folder", "Install", "Quick setup", "Ready"] as const;

export function Wizard() {
  const { setWizard, setPage } = useStore();
  const state = useLauncherState();
  const [step, setStep] = useState(0);
  const [folder, setFolder] = useState(state.data?.gameFolder ?? "");
  const client = useQueryClient();
  const returning = !!state.data?.gameFolder;

  useEffect(() => setWizard(true), [setWizard]);

  // Remember the folder here too: "Skip" on an up-to-date installation never calls install_patch (which saves it), and
  // without a saved folder the app would show this wizard again on every start.
  const finish = async () => {
    const current = await api.getState();
    if (current.gameFolder !== folder) await api.setState({ ...current, gameFolder: folder });
    await client.invalidateQueries();
    setPage("home");
    setWizard(false);
  };

  return (
    <div className="relative h-full overflow-y-auto">
      <Backdrop />
      <div className="relative mx-auto flex min-h-full max-w-3xl flex-col px-8 py-10">
        <header className="mb-8 flex items-center gap-4">
          <Mark className="size-12" />
          <div>
            <h1 className="font-display text-2xl text-ivory-50">Black &amp; White Launcher</h1>
            <p className="text-sm text-ivory-400">Set up the decompilation for your copy of the game</p>
          </div>
          {returning && (
            <Button variant="ghost" className="ml-auto" onClick={() => setWizard(false)}>Cancel</Button>
          )}
        </header>

        <ol className="mb-8 flex gap-2" aria-label="Steps">
          {STEPS.map((name, i) => (
            <li key={name} className="flex-1">
              <div className={cx("h-1 rounded-full", i <= step ? "bg-gold-400" : "bg-ink-700")} />
              <div className={cx("mt-2 text-xs", i === step ? "text-ivory-50" : "text-ink-500")}>{name}</div>
            </li>
          ))}
        </ol>

        <div className="flex-1">
          {step === 0 && <Welcome onNext={() => setStep(1)} />}
          {step === 1 && <Locate folder={folder} setFolder={setFolder} onBack={() => setStep(0)} onNext={() => setStep(2)} />}
          {step === 2 && <Install folder={folder} onBack={() => setStep(1)} onNext={() => setStep(3)} />}
          {step === 3 && <QuickSetup folder={folder} onNext={() => setStep(4)} />}
          {step === 4 && <Ready folder={folder} onDone={finish} />}
        </div>
      </div>
    </div>
  );
}

function Nav({ onBack, onNext, nextLabel = "Continue", nextDisabled, busy }: {
  onBack?: () => void; onNext: () => void; nextLabel?: string; nextDisabled?: boolean; busy?: boolean;
}) {
  return (
    <div className="mt-6 flex justify-between">
      {onBack ? <Button variant="ghost" icon={<ArrowLeft className="size-4" />} onClick={onBack}>Back</Button> : <span />}
      <Button variant="primary" onClick={onNext} disabled={nextDisabled} busy={busy}>
        {nextLabel} <ArrowRight className="size-4" />
      </Button>
    </div>
  );
}

function Welcome({ onNext }: { onNext: () => void }) {
  const info = useAppInfo();
  return (
    <Card>
      <h2 className="font-display text-xl text-ivory-50">Welcome</h2>
      <p className="mt-3 text-sm leading-relaxed text-ivory-300">
        This launcher installs the <strong className="text-ivory-50">Black &amp; White decompilation</strong>{" "}
        {info.data?.decompVersion && <Badge tone="gold">{info.data.decompVersion}</Badge>} into your installed copy of
        the game. It runs Lionhead's 2001 game on current Windows with its old problems fixed (SafeDisc, the
        DirectX&nbsp;7 check, crashes at high resolutions, hangs on quitting, the disc requirement) and adds a
        Direct3D&nbsp;9 renderer with widescreen resolutions, windowed modes, anti-aliasing and texture packs.
      </p>
      <ul className="mt-4 space-y-2 text-sm text-ivory-300">
        {[
          "Adds BWLaunch.exe, BWDecomp.dll, BWDecomp.ini and a BWDecomp folder with the documentation.",
          "Changes none of the game's own files, so the game can always be started as it was.",
          "Needs Black & White 1.00 (English) with the unprotected runblack.exe from the disc's crack folder.",
        ].map((t) => (
          <li key={t} className="flex gap-2"><Check className="mt-0.5 size-4 shrink-0 text-good-400" />{t}</li>
        ))}
      </ul>
      {!info.data?.hasPayload && (
        <div className="mt-4">
          <Notice kind="error" title="This launcher has no decompilation bundled">
            It was built without running scripts/sync-payload.ps1, so it cannot install anything.
          </Notice>
        </div>
      )}
      <Nav onNext={onNext} nextLabel="Get started" />
    </Card>
  );
}

function Locate({ folder, setFolder, onBack, onNext }: {
  folder: string; setFolder: (f: string) => void; onBack: () => void; onNext: () => void;
}) {
  const [typed, setTyped] = useState(folder);
  const [detecting, setDetecting] = useState(false);
  const [anyway, setAnyway] = useState(false);
  const check = useGameCheck(folder || null);

  const detect = async () => {
    setDetecting(true);
    try {
      const found = await api.detectGameFolder();
      if (found) {
        setTyped(found);
        setFolder(found);
      }
    } finally {
      setDetecting(false);
    }
  };
  useEffect(() => {
    if (!folder) void detect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Check what is typed after a short pause, as setup does.
  useEffect(() => {
    const t = setTimeout(() => setFolder(typed.trim()), 400);
    return () => clearTimeout(t);
  }, [typed, setFolder]);

  const browse = async () => {
    const picked = await open({ directory: true, title: "The Black & White folder (with runblack.exe)", defaultPath: folder || undefined });
    if (typeof picked === "string") {
      setTyped(picked);
      setFolder(picked);
    }
  };

  const build = check.data?.build;
  const canContinue = build === "supported" || ((build === "unknown" || build === "retailSafeDisc") && anyway);
  return (
    <Card>
      <h2 className="font-display text-xl text-ivory-50">Where is Black &amp; White installed?</h2>
      <p className="mt-2 text-sm text-ivory-400">The folder that contains runblack.exe.</p>
      <div className="mt-4 flex gap-2">
        <input className={cx(inputClass, "flex-1")} value={typed} placeholder="e.g. C:\Program Files (x86)\Lionhead Studios Ltd\Black & White"
          onChange={(e) => setTyped(e.target.value)} spellCheck={false} aria-label="Game folder" />
        <Button icon={<FolderOpen className="size-4" />} onClick={browse}>Browse</Button>
        <Button variant="ghost" icon={<Search className="size-4" />} onClick={detect} busy={detecting}>Find</Button>
      </div>
      <div className="mt-4 min-h-16">
        {!folder && !detecting && <Notice>Could not find the game automatically. Choose its folder.</Notice>}
        {folder && check.isFetching && <Notice>Checking runblack.exe…</Notice>}
        {folder && check.error && <Notice kind="error">{errorText(check.error)}</Notice>}
        {check.data && !check.isFetching && (
          build === "supported" ? (
            <Notice kind="success" title="Supported game found">Black &amp; White 1.00, unprotected runblack.exe.</Notice>
          ) : (
            <div className="space-y-3">
              <Notice kind={build === "missing" ? "info" : "warning"}
                title={build === "retailSafeDisc" ? "SafeDisc-protected runblack.exe" : build === "unknown" ? "Unsupported runblack.exe" : undefined}>
                {check.data.message}
              </Notice>
              {(build === "unknown" || build === "retailSafeDisc") && (
                <label className="flex items-center gap-2 text-sm text-ivory-300">
                  <input type="checkbox" checked={anyway} onChange={(e) => setAnyway(e.target.checked)} />
                  Install anyway (BWLaunch.exe will refuse to start this runblack.exe until it is replaced)
                </label>
              )}
            </div>
          )
        )}
      </div>
      <Nav onBack={onBack} onNext={onNext} nextDisabled={!canContinue} />
    </Card>
  );
}

function Install({ folder, onBack, onNext }: { folder: string; onBack: () => void; onNext: () => void }) {
  const status = usePatchStatus(folder);
  const client = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const s = status.data;

  const install = async () => {
    setBusy(true);
    setError("");
    try {
      await api.installPatch(folder);
      await client.invalidateQueries({ queryKey: keys.patch(folder) });
      await client.invalidateQueries({ queryKey: keys.state });
      onNext();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const upToDate = s?.state === "installed";
  return (
    <Card>
      <h2 className="font-display text-xl text-ivory-50">{upToDate ? "Already installed" : "Install the decompilation"}</h2>
      <p className="mt-2 break-all text-sm text-ivory-400">{folder}</p>
      <div className="mt-4 space-y-3">
        {s?.state === "notInstalled" && <Notice>Nothing is installed in this folder yet.</Notice>}
        {s?.state === "outdated" && (
          <Notice title={`Version ${s.installedVersion || "?"} is installed`}>
            It will be updated to {s.bundledVersion}. Your BWDecomp.ini is kept.
          </Notice>
        )}
        {upToDate && <Notice kind="success" title={`Version ${s.installedVersion} is installed`}>You can reinstall it to repair missing files.</Notice>}
        {s?.state === "byHand" && (
          <Notice title="Files copied by hand were found">
            BWDecomp.dll is there without an installation record. Installing replaces the program files and keeps your BWDecomp.ini.
          </Notice>
        )}
        {s?.fromSetup && (
          <Notice title="Installed by the setup program">
            The launcher takes this installation over; the old uninstaller and its Settings › Apps entry are removed.
          </Notice>
        )}
        {s?.needsRights && (
          <Notice kind="warning" title="Administrator rights needed">
            This folder is protected (for example below Program Files). Windows will ask for permission to copy the files.
          </Notice>
        )}
        {s?.running && <Notice kind="error">Close Black &amp; White first: it is running from this folder.</Notice>}
        {error && <Notice kind="error" title="Installation failed">{error}</Notice>}
      </div>
      <div className="mt-6 flex justify-between">
        <Button variant="ghost" icon={<ArrowLeft className="size-4" />} onClick={onBack}>Back</Button>
        <div className="flex gap-2">
          {upToDate && <Button onClick={onNext}>Skip</Button>}
          <Button variant="primary" onClick={install} busy={busy} disabled={!s || s.running}
            icon={s?.needsRights ? <ShieldAlert className="size-4" /> : undefined}>
            {upToDate ? "Reinstall" : s?.state === "outdated" ? "Update" : "Install"}
          </Button>
        </div>
      </div>
    </Card>
  );
}

function QuickSetup({ folder, onNext }: { folder: string; onNext: () => void }) {
  const [snapshot, setSnapshot] = useState<SettingsSnapshot | null>(null);
  const [values, setValues] = useState<Values>({});
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const original = useMemo(() => (snapshot ? toValues(snapshot) : {}), [snapshot]);

  useEffect(() => {
    void Promise.all([api.readSettings(folder), api.displayModes()]).then(([s, modes]) => {
      setSnapshot(s);
      const v = toValues(s);
      // No resolution yet: suggest the display's own (the game would use 800×600).
      if (!v["game.ScreenW"] && modes.current) {
        v["game.ScreenW"] = String(modes.current.width);
        v["game.ScreenH"] = String(modes.current.height);
        v["game.ScreenD"] = "32";
      }
      setValues(v);
    }, (e) => setError(errorText(e)));
  }, [folder]);

  const save = async () => {
    setBusy(true);
    setError("");
    try {
      await api.writeSettings(folder, toChange(original, values));
      onNext();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const onChange = (id: string, v: string) => setValues((old) => ({ ...old, [id]: v }));
  const h = hints(values);
  return (
    <Card>
      <h2 className="font-display text-xl text-ivory-50">Quick setup</h2>
      <p className="mt-2 text-sm text-ivory-400">The essentials. Everything else is under Settings.</p>
      {!snapshot && !error && <p className="mt-6 text-sm text-ivory-400">Reading settings…</p>}
      {snapshot && (
        <div className="mt-4">
          <ResolutionField values={values} onChange={onChange} hint={h["game.ScreenW"]} />
          {FIELDS.filter((f) => f.quick).map((f) => (
            <SettingField key={f.id} field={f} values={values} onChange={onChange} hint={h[f.id]} />
          ))}
        </div>
      )}
      {error && <div className="mt-4"><Notice kind="error">{error}</Notice></div>}
      <Nav onNext={save} nextLabel="Save" busy={busy} nextDisabled={!snapshot} />
    </Card>
  );
}

function Ready({ folder, onDone }: { folder: string; onDone: () => void }) {
  const [error, setError] = useState("");
  const play = async () => {
    try {
      await api.launchGame(folder);
      useStore.getState().setRun("starting");
      onDone();
    } catch (e) {
      setError(errorText(e));
    }
  };
  return (
    <Card>
      <div className="flex flex-col items-center py-6 text-center">
        <Mark className="size-20" />
        <h2 className="mt-5 font-display text-2xl text-ivory-50">Ready to play</h2>
        <p className="mt-2 max-w-md text-sm text-ivory-400">
          Start the game from here at any time. Settings, addons and updates are in the launcher.
        </p>
        {error && <div className="mt-4 w-full"><Notice kind="error">{error}</Notice></div>}
        <div className="mt-6 flex gap-3">
          <Button onClick={onDone}>Open the launcher</Button>
          <Button variant="primary" size="lg" icon={<Play className="size-5" />} onClick={play}>Play</Button>
        </div>
      </div>
    </Card>
  );
}
