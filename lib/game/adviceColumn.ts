import { rollSeeded } from "./seededRng";

// ── Types ────────────────────────────────────────────────────────────

export type ProblemCategory =
  | "hunger"
  | "overwork"
  | "loneliness"
  | "health"
  | "territory"
  | "leadership";

export type EmotionalTone = "desperate" | "worried" | "curious" | "grumpy";

export interface AdviceEntry {
  catName: string;
  problemCategory: ProblemCategory;
  tone: EmotionalTone;
  letterBody: string;
  adviceResponse: string;
}

export interface AdviceColumn {
  sectionTitle: string;
  entries: string[];
  totalEntries: number;
  editionNumber: number;
}

// ── Internal Data ────────────────────────────────────────────────────

interface CatNeeds {
  hunger: number;
  rest: number;
  health: number;
}

const NEED_THRESHOLD = 40;

const STATUS_PROBLEM_MAP: Record<string, ProblemCategory> = {
  critical: "territory",
  struggling: "loneliness",
  new: "leadership",
};

const LETTER_TEMPLATES: Record<
  ProblemCategory,
  Record<EmotionalTone, string[]>
> = {
  hunger: {
    desperate: [
      "Dear Tabby, I haven't eaten properly in days. My belly rumbles louder than a thunderstorm. Please help! — {name}",
      "Dear Tabby, the food bowls are EMPTY and nobody seems to care! I'm wasting away here! — {name}",
      "Dear Tabby, I dream of mice every night but wake up to nothing. I fear I won't last much longer. — {name}",
    ],
    worried: [
      "Dear Tabby, meals have been getting smaller lately and I'm starting to worry. What should I do? — {name}",
      "Dear Tabby, I noticed the food stores are running low. Should I be concerned? — {name}",
      "Dear Tabby, my portions keep shrinking. Is the colony in trouble? — {name}",
    ],
    curious: [
      "Dear Tabby, I've been thinking about trying new food sources. Any suggestions for a hungry cat? — {name}",
      "Dear Tabby, what's the best way to find extra snacks around the colony? Asking for a friend. — {name}",
      "Dear Tabby, is it normal to think about food this much? Just wondering. — {name}",
    ],
    grumpy: [
      "Dear Tabby, who's in charge of the food around here? Because they're doing a terrible job. — {name}",
      "Dear Tabby, I shouldn't have to hunt on an empty stomach. This is ridiculous. — {name}",
      "Dear Tabby, back in my day, the food bowls were always full. What happened? — {name}",
    ],
  },
  overwork: {
    desperate: [
      "Dear Tabby, I've been on patrol for what feels like forever. My paws are bleeding. I can't go on. — {name}",
      "Dear Tabby, they keep assigning me tasks and I haven't slept in ages. I'm breaking down! — {name}",
      "Dear Tabby, I collapsed during my shift today. Nobody noticed. Please help me. — {name}",
    ],
    worried: [
      "Dear Tabby, I seem to be working more than everyone else. Is this normal? — {name}",
      "Dear Tabby, my task list keeps growing and I never get a break. What should I do? — {name}",
      "Dear Tabby, I'm tired all the time but don't want to let the colony down. Any advice? — {name}",
    ],
    curious: [
      "Dear Tabby, how many tasks is too many tasks? I've lost count of mine. — {name}",
      "Dear Tabby, what's the ideal work-nap ratio for a productive cat? — {name}",
      "Dear Tabby, is it possible to love your work AND want a nap? — {name}",
    ],
    grumpy: [
      "Dear Tabby, why am I doing three cats' worth of work? This is exploitation! — {name}",
      "Dear Tabby, some cats nap all day while I slave away. How is this fair? — {name}",
      "Dear Tabby, I demand better working conditions. The colony council is useless. — {name}",
    ],
  },
  loneliness: {
    desperate: [
      "Dear Tabby, I'm the only cat in my part of the colony. The silence is crushing me. — {name}",
      "Dear Tabby, nobody talks to me anymore. I feel invisible. Does anyone even care? — {name}",
      "Dear Tabby, I've forgotten what it feels like to groom someone. I'm so alone. — {name}",
    ],
    worried: [
      "Dear Tabby, the colony seems emptier lately. I miss having friends around. — {name}",
      "Dear Tabby, I try to be friendly but the other cats seem too busy. Am I doing something wrong? — {name}",
      "Dear Tabby, there used to be more of us. Where did everyone go? — {name}",
    ],
    curious: [
      "Dear Tabby, how do you make friends in a small colony? Asking for myself. — {name}",
      "Dear Tabby, is it weird to talk to the birds? They're my only company some days. — {name}",
      "Dear Tabby, what's the best way to attract new cats to our colony? — {name}",
    ],
    grumpy: [
      "Dear Tabby, the colony is too small and too quiet. Somebody fix this. — {name}",
      "Dear Tabby, I didn't sign up for a hermit lifestyle. Where are all the cats? — {name}",
      "Dear Tabby, a colony of two is barely a colony at all. This is unacceptable. — {name}",
    ],
  },
  health: {
    desperate: [
      "Dear Tabby, I can barely walk. My wounds aren't healing and I fear the worst. — {name}",
      "Dear Tabby, I've been sick for so long I can't remember being well. Is this the end? — {name}",
      "Dear Tabby, every bone in my body aches. I need help urgently! — {name}",
    ],
    worried: [
      "Dear Tabby, I've been feeling under the weather lately. Should I see the colony healer? — {name}",
      "Dear Tabby, my old injury is acting up again. I'm worried it might get worse. — {name}",
      "Dear Tabby, I keep getting tired more easily than before. Is something wrong with me? — {name}",
    ],
    curious: [
      "Dear Tabby, what herbs are good for general cat wellness? I want to stay healthy. — {name}",
      "Dear Tabby, is it true that purring helps you heal faster? Curious minds want to know. — {name}",
      "Dear Tabby, what's the secret to a long and healthy cat life? — {name}",
    ],
    grumpy: [
      "Dear Tabby, the colony's health care is abysmal. My scratches never heal properly. — {name}",
      "Dear Tabby, why don't we have a proper medicine cat? This is a disgrace. — {name}",
      "Dear Tabby, I'm sick of being sick. Somebody needs to sort out the herb supplies. — {name}",
    ],
  },
  territory: {
    desperate: [
      "Dear Tabby, predators are everywhere! I can't even leave the den without fearing for my life! — {name}",
      "Dear Tabby, we've been attacked three times this week. The colony isn't safe anymore! — {name}",
      "Dear Tabby, I hear growling in the bushes every night. I'm terrified to sleep. — {name}",
    ],
    worried: [
      "Dear Tabby, I've noticed more predator tracks near the colony. Should we be concerned? — {name}",
      "Dear Tabby, the patrols seem more dangerous lately. Are we losing territory? — {name}",
      "Dear Tabby, a fox was spotted near the den. What precautions should we take? — {name}",
    ],
    curious: [
      "Dear Tabby, how big should a colony's territory be? Ours feels small. — {name}",
      "Dear Tabby, what's the best way to mark territory so predators stay away? — {name}",
      "Dear Tabby, is it true that larger colonies have fewer predator problems? — {name}",
    ],
    grumpy: [
      "Dear Tabby, the so-called 'guards' are useless. I could do a better job sleeping. — {name}",
      "Dear Tabby, we need better defences and we need them NOW. — {name}",
      "Dear Tabby, back when I was young we never had this many predators. Someone's not doing their job. — {name}",
    ],
  },
  leadership: {
    desperate: [
      "Dear Tabby, the colony has no direction! Without proper leadership we're doomed! — {name}",
      "Dear Tabby, nobody's making decisions and everything is falling apart! We need a leader NOW! — {name}",
      "Dear Tabby, the council is in chaos. Cats are fighting and nothing gets done. Save us! — {name}",
    ],
    worried: [
      "Dear Tabby, I'm not sure our leader is making the best decisions. What can we do? — {name}",
      "Dear Tabby, the colony seems directionless lately. Should someone speak up? — {name}",
      "Dear Tabby, our leader hasn't been around much. Is the colony going to be okay? — {name}",
    ],
    curious: [
      "Dear Tabby, what makes a good colony leader? I've been thinking about it lately. — {name}",
      "Dear Tabby, how are leaders chosen in other colonies? Just curious. — {name}",
      "Dear Tabby, is it normal for new colonies to struggle with leadership? — {name}",
    ],
    grumpy: [
      "Dear Tabby, the leader couldn't organise a hunt in a mouse den. We deserve better. — {name}",
      "Dear Tabby, I've seen kittens with better decision-making skills than our council. — {name}",
      "Dear Tabby, leadership should be earned, not given. The current lot are useless. — {name}",
    ],
  },
};

const ADVICE_TEMPLATES: Record<
  ProblemCategory,
  Record<EmotionalTone, string[]>
> = {
  hunger: {
    desperate: [
      "Dear one, hunger is the hardest trial. Rally your colony to organise a group hunt — many paws make light work. Stay strong. — Yours, Tabby",
      "Sweet cat, your survival comes first. Seek out the herb garden and forage what you can while hunts are planned. — With care, Tabby",
      "Poor dear, no cat should go hungry. Speak to the colony leader about emergency rations immediately. — Urgently, Tabby",
    ],
    worried: [
      "Worry not too much, dear. Lean times come and go. Focus on building food stores when you can. — Warmly, Tabby",
      "A wise cat plans ahead. Start caching small portions now for harder days. — Practically, Tabby",
      "These concerns are valid, dear. Suggest a rationing system to your colony mates. — Sensibly, Tabby",
    ],
    curious: [
      "What a delightful question! Try exploring new hunting grounds — you might discover untapped prey. — Cheerfully, Tabby",
      "Variety is the spice of cat life! Consider learning to fish if you haven't already. — Adventurously, Tabby",
      "Every cat thinks about food, dear. It's perfectly natural. Try the eastern meadow for voles. — Helpfully, Tabby",
    ],
    grumpy: [
      "Your frustration is understandable, but complaining won't fill bowls. Volunteer for a hunt instead. — Firmly, Tabby",
      "Channel that grumpiness into action, dear. Organise a foraging party yourself. — Pointedly, Tabby",
      "Nostalgia won't feed the colony. Times change — adapt and help gather. — Bluntly, Tabby",
    ],
  },
  overwork: {
    desperate: [
      "Dear exhausted one, you MUST rest. No task is worth your health. Tell your leader you need recovery time. — Insistently, Tabby",
      "Collapse is your body's final warning. Stop working immediately and find a warm, quiet spot. — Urgently, Tabby",
      "Sweet cat, martyrdom helps no one. A rested cat works twice as well. Demand a break. — Firmly, Tabby",
    ],
    worried: [
      "Balance is key, dear. Talk to your task assigner about a fairer distribution of duties. — Wisely, Tabby",
      "Feeling overwhelmed is a sign to delegate. You can't do everything alone. — Gently, Tabby",
      "A tired cat makes mistakes. Schedule proper nap breaks between your duties. — Practically, Tabby",
    ],
    curious: [
      "The ideal ratio? Three parts nap, one part work, dear. At least that's what I recommend! — Playfully, Tabby",
      "You can absolutely love your work and want a nap. Both are noble cat pursuits. — Amusedly, Tabby",
      "If you've lost count, that's your answer — it's too many! Prune your task list. — Cheerfully, Tabby",
    ],
    grumpy: [
      "You're right to be upset. Fair work distribution matters. Bring it up at the next colony meeting. — Supportively, Tabby",
      "Some cats are indeed lazy, but shaming won't help. Lead by example and set boundaries. — Diplomatically, Tabby",
      "Demanding better conditions is your right. Put it in writing to the council. — Encouragingly, Tabby",
    ],
  },
  loneliness: {
    desperate: [
      "Oh dear heart, loneliness is a deep wound. Seek out any cat, even briefly. One conversation can change everything. — With love, Tabby",
      "You are not invisible, I promise. Start small — sit near another cat during meals. Presence matters. — Tenderly, Tabby",
      "Sweet lonely soul, grooming is a gift you can offer. Approach someone gently. Connection heals. — Warmly, Tabby",
    ],
    worried: [
      "Colony life ebbs and flows, dear. Make an effort to join group activities. Friends find friends. — Encouragingly, Tabby",
      "You're not doing anything wrong. Some cats are just busy. Persistence in friendship always pays off. — Reassuringly, Tabby",
      "Colonies grow and shrink naturally. Cherish the cats who are here now. — Philosophically, Tabby",
    ],
    curious: [
      "Making friends starts with sharing a sunbeam! Sit near others and let conversation happen naturally. — Cheerfully, Tabby",
      "Talking to birds is perfectly acceptable, dear. But do try the cat next door too. — Amusedly, Tabby",
      "New cats are attracted to happy colonies. Focus on making your home warm and welcoming. — Helpfully, Tabby",
    ],
    grumpy: [
      "Instead of complaining about the quiet, be the noise! Organise a gathering. — Pointedly, Tabby",
      "A colony is what you make of it, dear. Start a tradition and others will join. — Firmly, Tabby",
      "Two determined cats can build something wonderful. Start with what you have. — Practically, Tabby",
    ],
  },
  health: {
    desperate: [
      "Dear suffering one, please rest immediately. Your body needs time to heal. Seek herbal remedies. — Urgently, Tabby",
      "Health is everything, dear. Stop all duties and focus solely on recovery. This is not optional. — Insistently, Tabby",
      "Every cat heals differently, but all cats need rest. Find a safe, warm den and stay there. — With concern, Tabby",
    ],
    worried: [
      "A colony healer can put your mind at ease. Don't ignore persistent symptoms. — Wisely, Tabby",
      "Old injuries need attention before they worsen. Rest the affected area and seek herbs. — Practically, Tabby",
      "Fatigue can signal many things. Eat well, sleep more, and monitor how you feel. — Gently, Tabby",
    ],
    curious: [
      "Chamomile and catnip are wonderful for general wellness! A daily grooming routine helps too. — Cheerfully, Tabby",
      "Purring does indeed have healing properties! Science and cat wisdom agree on this one. — Delightedly, Tabby",
      "The secret to a long life? Eat well, sleep often, avoid unnecessary fights, and always purr. — Sagely, Tabby",
    ],
    grumpy: [
      "You're right — every colony needs a dedicated healer. Raise this issue with leadership. — Supportively, Tabby",
      "Herb supplies should be a priority. Volunteer to help gather them if no one else will. — Practically, Tabby",
      "Being sick of being sick is the best motivation to act. Push for better colony healthcare. — Encouragingly, Tabby",
    ],
  },
  territory: {
    desperate: [
      "Dear frightened one, stay close to the den and travel in groups. Safety in numbers is real. — Urgently, Tabby",
      "Three attacks in a week demands immediate action. The colony must reinforce defences NOW. — Insistently, Tabby",
      "Fear is your ally — it keeps you alert. But the colony must address this threat together. — Firmly, Tabby",
    ],
    worried: [
      "Vigilance is wise, not paranoid. Report all tracks to the patrol leader and stay alert. — Sensibly, Tabby",
      "Territory shifts are natural. Double up patrols and mark boundaries more frequently. — Practically, Tabby",
      "A fox sighting deserves caution but not panic. Avoid the area and inform all cats. — Calmly, Tabby",
    ],
    curious: [
      "Territory size depends on your colony's needs. Quality of land matters more than quantity. — Wisely, Tabby",
      "Scent marking is the first line of defence! Refresh marks daily at territory borders. — Helpfully, Tabby",
      "Larger colonies do deter predators, yes. Strength in numbers is nature's way. — Informatively, Tabby",
    ],
    grumpy: [
      "Guards need proper training and rotation. Suggest a training programme to leadership. — Practically, Tabby",
      "Frustration is valid, but action is better. Volunteer for the defence committee. — Pointedly, Tabby",
      "Every generation faces new challenges. Adapt your defences to current threats. — Firmly, Tabby",
    ],
  },
  leadership: {
    desperate: [
      "Dear worried one, every colony finds its way. Step up yourself if no one else will — leaders emerge in crisis. — Boldly, Tabby",
      "Chaos is temporary. Gather the most sensible cats and form an emergency council. — Urgently, Tabby",
      "Fighting solves nothing. Call a colony-wide meeting and let every cat have a voice. — Wisely, Tabby",
    ],
    worried: [
      "Leaders are cats too, and they make mistakes. Offer constructive suggestions rather than criticism. — Gently, Tabby",
      "A directionless colony needs goals. Suggest one clear priority for the colony to rally around. — Practically, Tabby",
      "Leadership absence is concerning. Organise tasks among yourselves until clarity returns. — Sensibly, Tabby",
    ],
    curious: [
      "A good leader listens more than they command. They put the colony's needs above their own. — Thoughtfully, Tabby",
      "Different colonies have different ways. Some elect, some inherit, some simply step forward. — Informatively, Tabby",
      "New colonies always struggle with leadership — it's perfectly normal. Give it time. — Reassuringly, Tabby",
    ],
    grumpy: [
      "Complaining about leaders is easy; leading is hard. If you can do better, prove it. — Bluntly, Tabby",
      "Harsh words, dear, but perhaps not unfair. Channel that energy into running for the council. — Pointedly, Tabby",
      "Earned leadership IS the ideal. Propose a merit-based system at the next assembly. — Encouragingly, Tabby",
    ],
  },
};

const PROBLEM_ORDINAL: Record<ProblemCategory, number> = {
  hunger: 0,
  overwork: 1,
  loneliness: 2,
  health: 3,
  territory: 4,
  leadership: 5,
};

const TONE_ORDINAL: Record<EmotionalTone, number> = {
  desperate: 0,
  worried: 1,
  curious: 2,
  grumpy: 3,
};

// Categories that produce "grumpy" tone in the medium severity range
const GRUMPY_CATEGORIES: Set<ProblemCategory> = new Set([
  "overwork",
  "territory",
  "leadership",
]);

// ── Public Functions ─────────────────────────────────────────────────

export function classifyProblem(
  catNeeds: CatNeeds,
  colonyStatus: string,
): ProblemCategory {
  // Find the worst need (lowest value)
  const needMap: [ProblemCategory, number][] = [
    ["hunger", catNeeds.hunger],
    ["overwork", catNeeds.rest],
    ["health", catNeeds.health],
  ];

  const worst = needMap.reduce((a, b) => (a[1] <= b[1] ? a : b));

  // If the worst need is below threshold, that's the problem
  if (worst[1] < NEED_THRESHOLD) {
    return worst[0];
  }

  // Otherwise, derive from colony status
  if (colonyStatus in STATUS_PROBLEM_MAP) {
    return STATUS_PROBLEM_MAP[colonyStatus];
  }

  // Fallback: hunger (there's always something to complain about regarding food)
  return "hunger";
}

export function getEmotionalTone(
  problemCategory: ProblemCategory,
  severityScore: number,
): EmotionalTone {
  if (severityScore >= 80) return "desperate";
  if (severityScore >= 50) return "worried";
  if (severityScore >= 30) {
    return GRUMPY_CATEGORIES.has(problemCategory) ? "grumpy" : "worried";
  }
  return "curious";
}

export function generateLetterBody(
  problemCategory: ProblemCategory,
  tone: EmotionalTone,
  catName: string,
  seed: number,
): string {
  const templates = LETTER_TEMPLATES[problemCategory][tone];
  const mixedSeed =
    seed * 7919 +
    PROBLEM_ORDINAL[problemCategory] * 131 +
    TONE_ORDINAL[tone] * 17;
  const { value } = rollSeeded(mixedSeed);
  const index = Math.floor(value * templates.length);
  return templates[index].replace("{name}", catName);
}

export function generateAdviceResponse(
  problemCategory: ProblemCategory,
  tone: EmotionalTone,
  seed: number,
): string {
  const templates = ADVICE_TEMPLATES[problemCategory][tone];
  const mixedSeed =
    seed * 6271 +
    PROBLEM_ORDINAL[problemCategory] * 197 +
    TONE_ORDINAL[tone] * 23;
  const { value } = rollSeeded(mixedSeed);
  const index = Math.floor(value * templates.length);
  return templates[index];
}

export function formatAdviceEntry(
  letterBody: string,
  adviceResponse: string,
  catName: string,
  problemCategory: ProblemCategory,
): string {
  const lines = [
    `--- Dear Tabby ---`,
    `Category: ${problemCategory}`,
    `From: ${catName}`,
    ``,
    letterBody,
    ``,
    `Tabby responds:`,
    adviceResponse,
  ];
  return lines.join("\n");
}

export function formatAdviceColumn(
  entries: string[],
  editionNumber: number,
): AdviceColumn {
  return {
    sectionTitle: `DEAR TABBY — Advice Column (Edition ${editionNumber})`,
    entries,
    totalEntries: entries.length,
    editionNumber,
  };
}
