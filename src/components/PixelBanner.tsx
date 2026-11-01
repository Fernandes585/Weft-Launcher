import "./pixel-banner.css";

/**
 * Cenário animado do banner (pixel art, fim de tarde).
 * Use como PRIMEIRO filho do banner. O banner precisa de position: relative e overflow: hidden,
 * e o conteúdo (título, botão Jogar) de position: relative e z-index: 1.
 */
export function PixelBanner() {
  return (
    <div className="pb" aria-hidden="true">
      <div className="pb-sun" />
      <div className="pb-layer pb-clouds-far" />
      <div className="pb-layer pb-hills-far" />
      <div className="pb-layer pb-clouds-near" />
      <div className="pb-layer pb-hills-mid" />
      <div className="pb-layer pb-trees" />
      <div className="pb-layer pb-ground" />
      <div className="pb-shade" />
    </div>
  );
}

export default PixelBanner;
