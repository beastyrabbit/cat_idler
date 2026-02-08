export interface CatTraitSummary {
  lineage: string;
  coat: string;
  eyes: string;
  markings: string;
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
