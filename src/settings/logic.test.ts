import { describe, expect, it } from "vite-plus/test";
import type { SettingsSnapshot } from "@/lib/api";
import { hints, isEmptyChange, toChange, toValues, vkFromKey, vkName } from "./logic";

const snapshot: SettingsSnapshot = {
  ini: [{ section: "display", key: "windowmode", value: "1" }],
  iniExists: true,
  defaults: [
    { section: "Display", key: "WindowMode", value: "0" },
    { section: "Display", key: "Renderer", value: "1" },
  ],
  game: { ScreenW: 1920, ScreenH: 1080, detailidx: 3 },
};

describe("settings logic", () => {
  it("reads the file, then the defaults, case-insensitively", () => {
    const v = toValues(snapshot);
    expect(v["Display.WindowMode"]).toBe("1");
    expect(v["Display.Renderer"]).toBe("1");
    expect(v["game.ScreenW"]).toBe("1920");
    expect(v["game.detailidx"]).toBe("3");
    expect(toValues(snapshot, true)["Display.WindowMode"]).toBe("0");
  });

  it("writes only what changed", () => {
    const before = toValues(snapshot);
    expect(isEmptyChange(toChange(before, before))).toBe(true);
    const after = { ...before, "Display.VSync": "0", "game.ScreenW": "2560", "game.ScreenH": "1440" };
    const change = toChange(before, after);
    expect(change.ini).toEqual([{ section: "Display", key: "VSync", value: "0" }]);
    expect(change.game).toEqual({ ScreenW: 2560, ScreenH: 1440 });
  });

  it("explains interactions", () => {
    const v = toValues(snapshot);
    expect(hints({ ...v, "Display.WindowMode": "0" })["Display.WindowMode"]).toMatch(/borderless/);
    const original = { ...v, "Display.Renderer": "0", "game.ScreenW": "2560" };
    expect(hints(original)["Display.AntiAliasing"]).toMatch(/Direct3D 9/);
    expect(hints(original)["game.ScreenW"]).toMatch(/2048/);
  });

  it("maps keys to Windows virtual-key codes", () => {
    expect(vkFromKey("F12", "F12")).toBe(123);
    expect(vkFromKey("p", "KeyP")).toBe(80);
    expect(vkFromKey("Shift", "ShiftLeft")).toBeNull();
    expect(vkName(123)).toBe("F12");
    expect(vkName(0)).toBe("None");
  });
});
