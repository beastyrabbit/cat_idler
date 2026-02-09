export type SkillDomain =
  | "hunting"
  | "combat"
  | "gathering"
  | "building"
  | "healing";

export interface CatForMentorship {
  name: string;
  lifeStage: "kitten" | "young" | "adult" | "elder";
  skills: Record<SkillDomain, number>;
}

export interface MentorMatch {
  mentorName: string;
  apprenticeName: string;
  skillDomain: SkillDomain;
  effectiveness: number;
  xpBonus: number;
}

const MENTOR_ELIGIBLE_STAGES = new Set(["adult", "elder"]);
const APPRENTICE_ELIGIBLE_STAGES = new Set(["young", "adult"]);
const MIN_MENTOR_SKILL = 50;
const SKILL_GAP_MINIMUM = 10;
const ELDER_BONUS = 0.2;
const OVERLOAD_PENALTY = 0.15;
const MAX_EFFECTIVENESS = 2.0;
const BASE_XP_PER_SESSION = 20;

export function canMentor(
  mentor: CatForMentorship,
  apprentice: CatForMentorship,
  skillDomain: SkillDomain,
): boolean {
  if (!MENTOR_ELIGIBLE_STAGES.has(mentor.lifeStage)) return false;
  if (!APPRENTICE_ELIGIBLE_STAGES.has(apprentice.lifeStage)) return false;
  if (mentor.skills[skillDomain] < MIN_MENTOR_SKILL) return false;
  if (
    apprentice.skills[skillDomain] >=
    mentor.skills[skillDomain] - SKILL_GAP_MINIMUM
  )
    return false;
  return true;
}

export function calculateTrainingEffectiveness(
  mentorSkill: number,
  apprenticeSkill: number,
  mentorStage: "adult" | "elder",
  apprenticeCount: number,
): number {
  // Base: mentor's skill level / 100 (0.5-1.0 range for valid mentors)
  const base = mentorSkill / 100;

  // Skill gap bonus: larger gaps = faster learning, capped at 2x
  const gap = mentorSkill - apprenticeSkill;
  const gapBonus = Math.min(gap / 50, 2);

  let effectiveness = base * gapBonus;

  // Elder wisdom bonus: +20%
  if (mentorStage === "elder") {
    effectiveness *= 1 + ELDER_BONUS;
  }

  // Fatigue penalty: -15% per extra student beyond the first
  const extraStudents = Math.max(0, apprenticeCount - 1);
  effectiveness *= 1 - extraStudents * OVERLOAD_PENALTY;

  // Cap at 2.0, floor at 0
  return Math.min(
    MAX_EFFECTIVENESS,
    Math.max(0, Math.round(effectiveness * 100) / 100),
  );
}

export function calculateMentorXPBonus(
  effectiveness: number,
  baseXP: number,
): number {
  return Math.round(effectiveness * baseXP);
}

export function findBestMentor(
  cats: CatForMentorship[],
  apprentice: CatForMentorship,
  skillDomain: SkillDomain,
): MentorMatch | null {
  let bestMatch: MentorMatch | null = null;
  let bestEffectiveness = -1;

  for (const cat of cats) {
    if (!canMentor(cat, apprentice, skillDomain)) continue;

    const effectiveness = calculateTrainingEffectiveness(
      cat.skills[skillDomain],
      apprentice.skills[skillDomain],
      cat.lifeStage as "adult" | "elder",
      1, // assume 1 apprentice when finding best
    );

    if (effectiveness > bestEffectiveness) {
      bestEffectiveness = effectiveness;
      bestMatch = {
        mentorName: cat.name,
        apprenticeName: apprentice.name,
        skillDomain,
        effectiveness,
        xpBonus: calculateMentorXPBonus(effectiveness, BASE_XP_PER_SESSION),
      };
    }
  }

  return bestMatch;
}

export function generateAcademyReport(
  pairings: MentorMatch[],
  colonyName: string,
): string {
  const header = `=== ${colonyName} Academy Report ===`;

  if (pairings.length === 0) {
    return `${header}\n\nNo classes in session today. The academy halls stand quiet in ${colonyName}.`;
  }

  const lines = pairings.map(
    (p) =>
      `${p.mentorName} is teaching ${p.apprenticeName} the art of ${p.skillDomain} (effectiveness: ${p.effectiveness.toFixed(1)}, +${p.xpBonus} XP)`,
  );

  const summary =
    pairings.length === 1
      ? `1 mentoring session is underway in ${colonyName}.`
      : `${pairings.length} mentoring sessions are underway in ${colonyName}.`;

  return `${header}\n\n${summary}\n\n${lines.join("\n")}`;
}
