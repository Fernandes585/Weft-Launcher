import { PixelIcon } from "./PixelIcon";

// Paletas de cores temáticas

// Home: casinha de madeira e pedra
const HOME_PALETTE = {
  "#": "#8b5e3c",   // marrom madeira - estrutura principal
  "+": "#6b4226",   // marrom escuro madeira - telhado/sombra
  "1": "#7a7a7a",   // cinza pedra - fundação/parede de pedra
  "2": "#5a5a5a",   // cinza escuro pedra - sombra
  "3": "#d4a574",   // bege claro - porta/janela
  "4": "#b88a5a",   // bege médio - detalhes
  ".": "transparent",
};

// Versions: 3 blocos voxel - terra, pedra, madeira
const VERSIONS_PALETTE = {
  // Bloco de terra (marrom/verde)
  "#": "#5d4037",   // marrom terra - topo
  "+": "#4a3428",   // marrom escuro terra - lado
  "1": "#8d6e63",   // marrom claro terra - face
  "2": "#6d4c41",   // marrom médio terra
  // Bloco de pedra (cinza)
  "3": "#78909c",   // cinza azulado pedra - topo
  "4": "#546e7a",   // cinza escuro pedra - lado
  "5": "#90a4ae",   // cinza claro pedra - face
  "6": "#607d8b",   // cinza médio pedra
  // Bloco de madeira (marrom)
  "7": "#8d6e63",   // marrom madeira - topo
  "8": "#6d4c41",   // marrom escuro madeira - lado
  "9": "#a1887f",   // marrom claro madeira - face
  "a": "#795548",   // marrom médio madeira
  ".": "transparent",
};

// Notes: pergaminho bege com partes grossas vermelhas (onde segura)
const NOTES_PALETTE = {
  "#": "#d4a574",   // bege claro - papel principal
  "+": "#b88a5a",   // bege médio - sombra do papel
  "1": "#f5f5dc",   // bege bem claro - destaque papel
  "2": "#c9b896",   // bege amarelado - texto/linhas
  "3": "#c62828",   // vermelho forte - partes grossas (onde segura)
  "4": "#8b1a1a",   // vermelho escuro - sombra partes grossas
  "5": "#ef5350",   // vermelho claro - brilho partes grossas
  ".": "transparent",
};

// Casinha de madeira e pedra (16x16)
const HOME_ROWS = [
  "................",
  "......##......",
  ".....#++#.....",
  "....#++++#....",
  "...#++++++#...",
  "..#++++++++#..",
  ".#++++++++++#.",
  "#++++++++++++#",
  "#++++++++++++#",
  "#111111111111#",
  "#111111111111#",
  "#12333333321#",
  "#12333333321#",
  "#12344444321#",
  "#12344444321#",
  "##############",
];

// 3 blocos voxel: terra (esq), pedra (meio), madeira (dir) - estilo isométrico (16x16)
const VERSIONS_ROWS = [
  "..............",
  "......##......",
  ".....#++#.....",
  "....#1111#....",
  "...#112211#...",
  "..#11222211#..",
  ".#1122222211#.",
  "#112222222211#",
  "#334455554433#",
  "#334455665544#",
  "#334455667744#",
  "##3344556677##",
  "....##5566##..",
  "......77##....",
  "..............",
  "..............",
];

// Pergaminho bege com partes grossas vermelhas nas laterais (16x16)
const NOTES_ROWS = [
  "................",
  "..############..",
  "..############..",
  ".#111111111111#.",
  "#11111111111111#",
  "#1112222221111#",
  "#1112222221111#",
  "#1112222221111#",
  "#1112222221111#",
  "#1112222221111#",
  "#1112222221111#",
  "#1112222221111#",
  "#333444444333#",
  "#333555555333#",
  ".#33333333333#.",
  "..############..",
];

export const HomeIcon = () => <PixelIcon className="tab-icon" rows={HOME_ROWS} palette={HOME_PALETTE} />;
export const VersionsIcon = () => <PixelIcon className="tab-icon" rows={VERSIONS_ROWS} palette={VERSIONS_PALETTE} />;
export const NotesIcon = () => <PixelIcon className="tab-icon" rows={NOTES_ROWS} palette={NOTES_PALETTE} />;
