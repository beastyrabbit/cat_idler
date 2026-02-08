export interface CatTraitSummary {
  lineage: string;
  coat: string;
  eyes: string;
  markings: string;
}

export interface CatIdentitySummary extends CatTraitSummary {
  species: string;
  skin: string;
  sprite: string;
  accessories: string;
  scars: string;
}

export function summarizeCatTraits(spriteParams: Record<string, unknown> | null | undefined): CatTraitSummary {
  const p = (spriteParams ?? {}) as Record<string, unknown>;

  const lineage = typeof p.peltName === 'string' && p.peltName.trim() ? p.peltName : 'Unknown lineage';

  const colour = typeof p.colour === 'string' && p.colour.trim() ? p.colour : 'Unknown coat';
  const tint = typeof p.tint === 'string' && p.tint !== 'none' ? ` · ${p.tint}` : '';
  const coat = `${colour}${tint}`;

  const eye1 = typeof p.eyeColour === 'string' && p.eyeColour.trim() ? p.eyeColour : 'Unknown eyes';
  const eye2 = typeof p.eyeColour2 === 'string' && p.eyeColour2.trim() ? p.eyeColour2 : null;
  const eyes = eye2 ? `${eye1}/${eye2}` : eye1;

  const marks: string[] = [];
  if (p.isTortie === true) {
    marks.push('Tortie');
  }
  if (typeof p.whitePatches === 'string' && p.whitePatches.trim()) {
    marks.push(`White ${p.whitePatches}`);
  }
  if (typeof p.points === 'string' && p.points.trim()) {
    marks.push(`Points ${p.points}`);
  }

  return {
    lineage,
    coat,
    eyes,
    markings: marks.length > 0 ? marks.join(', ') : 'None',
  };
}

function valueOrFallback(params: Record<string, unknown>, key: string, fallback: string): string {
  const value = params[key];
  return typeof value === 'string' && value.trim() ? value : fallback;
}

export function summarizeCatIdentity(spriteParams: Record<string, unknown> | null | undefined): CatIdentitySummary {
  const p = (spriteParams ?? {}) as Record<string, unknown>;
  const traits = summarizeCatTraits(p);

  const spriteNumber =
    typeof p.spriteNumber === 'number' && Number.isFinite(p.spriteNumber)
      ? `#${Math.floor(p.spriteNumber)}`
      : 'Unknown';

  const accessoriesRaw = Array.isArray(p.accessories) ? p.accessories.filter((item) => typeof item === 'string') : [];
  const scarsRaw = Array.isArray(p.scars) ? p.scars.filter((item) => typeof item === 'string') : [];

  return {
    species: 'Domestic Cat',
    ...traits,
    skin: valueOrFallback(p, 'skinColour', 'Unknown'),
    sprite: spriteNumber,
    accessories: accessoriesRaw.length > 0 ? accessoriesRaw.join(', ') : 'None',
    scars: scarsRaw.length > 0 ? scarsRaw.join(', ') : 'None',
  };
}
