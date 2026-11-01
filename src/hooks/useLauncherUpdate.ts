import { useCallback, useEffect, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus = "idle" | "available" | "downloading" | "error";

export function useLauncherUpdate() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [percent, setPercent] = useState(0);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Ao abrir o launcher, procura uma versão nova. Sem internet ou no modo dev, ignora em silêncio.
  useEffect(() => {
    let cancelled = false;
    check()
      .then((u) => {
        if (!cancelled && u) {
          setUpdate(u);
          setStatus("available");
        }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const install = useCallback(async () => {
    if (!update) return;
    setStatus("downloading");
    setPercent(0);
    setErrorMsg(null);
    let total = 0;
    let done = 0;
    try {
      await update.downloadAndInstall((e) => {
        if (e.event === "Started") {
          total = e.data.contentLength ?? 0;
        } else if (e.event === "Progress") {
          done += e.data.chunkLength;
          if (total > 0) setPercent(Math.min(100, Math.round((done / total) * 100)));
        }
      });
      await relaunch();
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setStatus("error");
    }
  }, [update]);

  return { status, version: update?.version ?? null, percent, errorMsg, install };
}