// App-wide UI state: the current page and the game's run state (fed by the backend's "game" events).

import { create } from "zustand";
import type { GameEvent } from "./api";

export type Page = "home" | "settings" | "addons" | "updates" | "about";
export type RunState = "idle" | "starting" | "running";

type Store = {
  page: Page;
  setPage: (page: Page) => void;
  run: RunState;
  setRun: (run: RunState) => void;
  /** The last start that failed, with the end of BWDecomp.log. */
  failure: GameEvent | null;
  setFailure: (failure: GameEvent | null) => void;
  /** Set by "Change folder" on Home: shows the setup wizard again. */
  wizard: boolean;
  setWizard: (wizard: boolean) => void;
};

export const useStore = create<Store>((set) => ({
  page: "home",
  setPage: (page) => set({ page }),
  run: "idle",
  setRun: (run) => set({ run }),
  failure: null,
  setFailure: (failure) => set({ failure }),
  wizard: false,
  setWizard: (wizard) => set({ wizard }),
}));
