import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./SidebarFooter.css";

const DISCORD_URL = "https://discord.gg/aWECX7EPmV";

const DiscordIcon = () => (
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <path d="M21 15a2 2 0 0 1-2 2H8l-5 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
  </svg>
);

export function SidebarFooter() {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(""));
  }, []);

  return (
    <div className="sf">
      <button className="sf-link" onClick={() => openUrl(DISCORD_URL)}>
        <DiscordIcon />
        <span>Discord</span>
      </button>
      {version && <div className="sf-version">Launcher {version}</div>}
    </div>
  );
}