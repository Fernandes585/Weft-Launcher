import { createContext, useContext, useState, useCallback, useEffect, useMemo, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { translateError } from "../i18n/errors";

export interface InstalledVersion {
  tag: string;
  path: string;
  executable: string;
  installed_at: string;
}

export interface InstallProgress {
  tag: string;
  stage: "downloading" | "verifying" | "extracting" | "finalizing" | "done" | "error";
  percent?: number;
  downloaded?: number;
  total?: number;
  speedMbps?: number;
  message?: string;
}

interface InstallContextValue {
  installedVersions: InstalledVersion[];
  refreshInstalled: () => Promise<void>;
  install: (tag: string, assetUrl: string, assetName: string, expectedSha256: string | null) => Promise<void>;
  remove: (tag: string) => Promise<void>;
  getProgress: (tag: string) => InstallProgress | null;
  isInstalling: (tag: string) => boolean;
  isRemoving: (tag: string) => boolean;
  getError: (tag: string) => string | null;
  clearError: (tag: string) => void;
}

const InstallContext = createContext<InstallContextValue | null>(null);

export function InstallProvider({ children }: { children: ReactNode }) {
  const [installedVersions, setInstalledVersions] = useState<InstalledVersion[]>([]);
  const [progressByTag, setProgressByTag] = useState<Record<string, InstallProgress>>({});
  const [errorByTag, setErrorByTag] = useState<Record<string, string>>({});
  const [removingTags, setRemovingTags] = useState<Set<string>>(new Set());

  const refreshInstalled = useCallback(async () => {
    try {
      const versions = await invoke<InstalledVersion[]>("get_installed_versions");
      setInstalledVersions(versions);
    } catch (err) {
      console.error("Failed to load installed versions:", err);
    }
  }, []);

  useEffect(() => {
    refreshInstalled();
  }, [refreshInstalled]);

  // Single listener for all installs; only updates tags that are actively installing
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let mounted = true;

    listen<InstallProgress>("install-progress", (event) => {
      if (!mounted) return;
      const payload = event.payload;
      setProgressByTag((prev) =>
        payload.tag in prev ? { ...prev, [payload.tag]: payload } : prev
      );
    }).then((fn) => {
      if (mounted) unlisten = fn;
      else fn();
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  const clearError = useCallback((tag: string) => {
    setErrorByTag((prev) => {
      if (!(tag in prev)) return prev;
      const next = { ...prev };
      delete next[tag];
      return next;
    });
  }, []);

  const install = useCallback(async (tag: string, assetUrl: string, assetName: string, expectedSha256: string | null) => {
    clearError(tag);
    setProgressByTag((prev) => ({ ...prev, [tag]: { tag, stage: "downloading", percent: 0 } }));

    try {
      await invoke<InstalledVersion>("install_version", {
        tag,
        assetUrl,
        assetName,
        expectedSha256,
      });
      await refreshInstalled();
    } catch (err) {
      setErrorByTag((prev) => ({ ...prev, [tag]: translateError(err) }));
      throw err;
    } finally {
      setProgressByTag((prev) => {
        if (!(tag in prev)) return prev;
        const next = { ...prev };
        delete next[tag];
        return next;
      });
    }
  }, [clearError, refreshInstalled]);

  const remove = useCallback(async (tag: string) => {
    clearError(tag);
    setRemovingTags((prev) => new Set(prev).add(tag));

    try {
      await invoke("remove_version", { tag });
      await refreshInstalled();
    } catch (err) {
      setErrorByTag((prev) => ({ ...prev, [tag]: translateError(err) }));
      throw err;
    } finally {
      setRemovingTags((prev) => {
        if (!prev.has(tag)) return prev;
        const next = new Set(prev);
        next.delete(tag);
        return next;
      });
    }
  }, [clearError, refreshInstalled]);

  const getProgress = useCallback((tag: string) => progressByTag[tag] ?? null, [progressByTag]);
  const isInstalling = useCallback((tag: string) => tag in progressByTag, [progressByTag]);
  const isRemoving = useCallback((tag: string) => removingTags.has(tag), [removingTags]);
  const getError = useCallback((tag: string) => errorByTag[tag] ?? null, [errorByTag]);

  const value = useMemo<InstallContextValue>(
    () => ({
      installedVersions,
      refreshInstalled,
      install,
      remove,
      getProgress,
      isInstalling,
      isRemoving,
      getError,
      clearError,
    }),
    [
      installedVersions,
      refreshInstalled,
      install,
      remove,
      getProgress,
      isInstalling,
      isRemoving,
      getError,
      clearError,
    ],
  );

  return <InstallContext.Provider value={value}>{children}</InstallContext.Provider>;
}

export function useInstall(): InstallContextValue {
  const context = useContext(InstallContext);
  if (!context) {
    throw new Error("useInstall must be used within an InstallProvider");
  }
  return context;
}