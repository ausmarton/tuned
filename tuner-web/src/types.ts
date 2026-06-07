export type Direction = "flat" | "in_tune" | "sharp";

export interface Snapshot {
  pitchHz: number | null;
  centsOff: number | null;
  direction: Direction | null;
  nearestString: number | null;
  nearestStringName: string | null;
  confidence: number;
}

export interface StrumString {
  index: number;
  name: string;
  targetHz: number;
  detectedHz: number | null;
  centsOff: number | null;
  direction: Direction | null;
  confidence: number;
}

export interface StrumReport {
  strings: StrumString[];
}

export interface ChordCandidate {
  name: string;
  score: number;
}

export interface ChordResult {
  candidates: ChordCandidate[];
  best: ChordCandidate | null;
}

export const SUPPORTED_TUNINGS: ReadonlyArray<readonly [string, string]> = [
  ["guitar.standard", "Guitar — Standard (E A D G B E)"],
  ["bass.standard", "Bass — Standard (E A D G)"],
  ["guitarra.lisboa", "Guitarra Portuguesa — Lisboa"],
  ["guitarra.coimbra", "Guitarra Portuguesa — Coimbra"],
];
