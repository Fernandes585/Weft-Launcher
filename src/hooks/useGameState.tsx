import { createContext, useContext, useEffect, useState, useCallback, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface GameStateEvent {
  tag: string;
  state: "running" | "stopped";
  exitCode: number | null;
}

interface GameStateContextValue {
  runningTags: Set<string>;
  anyRunning: boolean;
  isRunning: (tag: string) => boolean;
  launch: (tag: string) => Promise<string | null>;
}

const GameStateContext = createContext<GameStateContextValue | null>(null);

export function GameStateProvider({ children }: { children: ReactNode }) {
  const [runningTags, setRunningTags] = useState<Set<string>>(new Set());

  const anyRunning = runningTags.size > 0;

  const isRunning = useCallback((tag: string) => runningTags.has(tag), [runningTags]);

  // Fetch initially running versions on mount
  useEffect(() => {
    let mounted = true;
    invoke<string[]>("get_running_versions")
      .then((tags) => {
        if (mounted) setRunningTags(new Set(tags));
      })
      .catch((err) => console.error("Failed to get running versions:", err));
    return () => {
      mounted = false;
    };
  }, []);

  // Listen for game-state events
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let mounted = true;

    listen<GameStateEvent>("game-state", (event) => {
      if (!mounted) return;
      const { tag, state } = event.payload;
      setRunningTags((prev) => {
        const next = new Set(prev);
        if (state === "running") {
          next.add(tag);
        } else {
          next.delete(tag);
        }
        return next;
      });
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      mounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  const launch = useCallback(async (tag: string): Promise<string | null> => {
    try {
      await invoke("launch_version", { tag });
      return null; // success
    } catch (err) {
      return err as string;
    }
  }, []);

  const value: GameStateContextValue = {
    runningTags,
    anyRunning,
    isRunning,
    launch,
  };

  return <GameStateContext.Provider value={value}>{children}</GameStateContext.Provider>;
}

export function useGameState(): GameStateContextValue {
  const context = useContext(GameStateContext);
  if (!context) {
    throw new Error("useGameState must be used within a GameStateProvider");
  }
  return context;
}