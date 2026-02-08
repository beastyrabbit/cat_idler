import { describe, expect, it } from 'vitest';

import { nextSpecialization, type CatSpecialization } from '@/lib/game/idleEngine';

function runRoleProgression(role: Exclude<CatSpecialization, null>, completions: number): CatSpecialization {
  let specialization: CatSpecialization = null;
  let roleXp = 0;

  for (let i = 0; i < completions; i += 1) {
    roleXp += 1;
    specialization = nextSpecialization(role, roleXp, specialization);
  }

  return specialization;
}

describe('specialization progression', () => {
  it('does not unlock before 10 completions and unlocks exactly at 10', () => {
    expect(runRoleProgression('hunter', 9)).toBeNull();
    expect(runRoleProgression('hunter', 10)).toBe('hunter');
  });

  it('supports all role unlock paths at the same threshold', () => {
    expect(runRoleProgression('hunter', 10)).toBe('hunter');
    expect(runRoleProgression('architect', 10)).toBe('architect');
    expect(runRoleProgression('ritualist', 10)).toBe('ritualist');
  });

  it('keeps existing specialization after unlock', () => {
    expect(nextSpecialization('hunter', 100, 'architect')).toBe('architect');
    expect(nextSpecialization('ritualist', 100, 'hunter')).toBe('hunter');
  });
});
