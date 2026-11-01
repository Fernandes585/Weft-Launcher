import { useState, useCallback, useEffect } from "react";
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

export function useInstall() {
  const [installedVersions, setInstalledVersions] = useState<InstalledVersion[]>([]);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

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

  const install = useCallback(async (tag: string, assetUrl: string, assetName: string, expectedSha256: string | null) => {
    setLoading(true);
    setError(null);
    setProgress({ tag, stage: "downloading", percent: 0 });

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<InstallProgress>("install-progress", (event) => {
        // Filter events by tag to avoid cross-contamination
        if (event.payload.tag === tag) {
          setProgress(event.payload);
        }
      });
    } catch {
      // Event listening failed, continue without progress updates
    }

    try {
      await invoke<InstalledVersion>("install_version", {
        tag,
        assetUrl,
        assetName,
        expectedSha256,
      });
      setProgress({ tag, stage: "done", percent: 100 });
      await refreshInstalled();
    } catch (err) {
      const translated = translateError(err);
      setError(translated);
      setProgress({ tag, stage: "error", message: translated });
      throw err;
    } finally {
      setLoading(false);
      if (unlisten) unlisten();
    }
  }, [refreshInstalled]);

  const remove = useCallback(async (tag: string) => {
    setLoading(true);
    setError(null);
    try {
      await invoke("remove_version", { tag });
      await refreshInstalled();
    } catch (err) {
      const translated = translateError(err);
      setError(translated);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [refreshInstalled]);

  const isInstalled = useCallback((tag: string) => {
    return installedVersions.some((v) => v.tag === tag);
  }, [installedVersions]);

  const clearError = useCallback(() => {
    setError(null);
    if (progress?.stage === "error") {
      setProgress(null);
    }
  }, [progress]);

  return {
    installedVersions,
    progress,
    error,
    loading,
    refreshInstalled,
    install,
    remove,
    isInstalled,
    clearError,
  };
}