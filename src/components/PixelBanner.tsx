import type { CSSProperties } from "react";
import "./pixel-banner.css";

/**
 * Cenário animado do banner (pixel art). Troca de bioma a cada 5 s:
 * planície ao entardecer, taiga com neve, deserto, floresta à noite e pântano com chuva.
 * Use como PRIMEIRO filho do banner. O banner precisa de position: relative e overflow: hidden,
 * e o conteúdo (título, botão Jogar) de position: relative e z-index: 1.
 * A ordem da lista define a ordem da troca (o CSS espera exatamente 5 cenas, 5 s cada).
 */
const BIOMES = [
  { id: "plains", body: true, stars: false, weather: null },
  { id: "taiga", body: false, stars: false, weather: "snow" },
  { id: "desert", body: true, stars: false, weather: null },
  { id: "night", body: true, stars: true, weather: null },
  { id: "swamp", body: false, stars: false, weather: "rain" },
] as const;

export function PixelBanner() {
  return (
    <div className="pb" aria-hidden="true">
      {BIOMES.map((b, i) => (
        <div
          key={b.id}
          className={`pb-scene pb-${b.id}`}
          style={{ "--i": i } as CSSProperties}
        >
          {b.body && <div className="pb-body" />}
          {b.stars && <div className="pb-stars" />}
          <div className="pb-layer pb-clouds-far" />
          <div className="pb-layer pb-hills-far" />
          <div className="pb-layer pb-clouds-near" />
          <div className="pb-layer pb-hills-mid" />
          <div className="pb-layer pb-trees" />
          <div className="pb-layer pb-ground" />
          {b.weather && <div className={`pb-weather pb-${b.weather}`} />}
        </div>
      ))}
      <div className="pb-shade" />
    </div>
  );
}

export default PixelBanner;
