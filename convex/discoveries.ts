/**
 * Discovery System
 * 
 * Handles accessory discovery during exploration and scar acquisition from combat.
 */

import { internalMutation } from "./_generated/server";
import { v } from "convex/values";
import { internal } from "./_generated/api";

/**
 * Common accessories that can be found during exploration
 */
const EXPLORATION_ACCESSORIES = [
  'FLOWER', 'FLOWER2', 'FLOWER3', 'FLOWER4', 'FLOWER5',
  'LEAF', 'LEAF2', 'LEAF3', 'LEAF4', 'LEAF5',
  'HERBS', 'HERBS2', 'HERBS3', 'HERBS4',
  'FEATHER', 'FEATHER2', 'FEATHER3',
  'STICK', 'STICK2', 'STICK3',
  'ROCK', 'ROCK2', 'ROCK3',
  'BERRY', 'BERRY2', 'BERRY3',
  'MOSS', 'MOSS2', 'MOSS3',
  'SHELL', 'SHELL2', 'SHELL3',
  'BONE', 'BONE2', 'BONE3',
];

/**
 * Rare accessories (lower chance to find)
 */
const RARE_ACCESSORIES = [
  'COLLAR', 'COLLAR2', 'COLLAR3', 'COLLAR4', 'COLLAR5',
  'BANDANA', 'BANDANA2', 'BANDANA3', 'BANDANA4', 'BANDANA5',
  'BOW', 'BOW2', 'BOW3', 'BOW4', 'BOW5',
  'BELL', 'BELL2', 'BELL3',
  'RING', 'RING2', 'RING3',
];

/**
 * Scars that can be acquired from combat
 */
const COMBAT_SCARS = [
  'ONE', 'TWO', 'THREE', 'MANY', 'BRIGHTHEART', 'NOTAIL',
  'HALFTAIL', 'LEFTEAR', 'RIGHTEAR', 'NOLEFTEAR', 'NORIGHTEAR',
  'NOEAR', 'BRIDGE', 'RIGHTBLIND', 'LEFTBLIND', 'BOTHBLIND',
  'BURNPAWS', 'BURNTAIL', 'BURNBELLY', 'BURNRUMP', 'FROSTFACE',
  'FROSTTAIL', 'FROSTPAW', 'FROSTMITT', 'FROSTSOCK',
];

/**
 * Discover an accessory during exploration
 * Called when an exploration task completes successfully
 */
export const discoverAccessory = internalMutation({
  args: {
    catId: v.id("cats"),
  },
  handler: async (ctx, args) => {
    const cat = await ctx.db.get(args.catId);
    if (!cat || cat.deathTime) {
      return null;
    }

    // 30% chance to find an accessory during exploration
    if (Math.random() > 0.3) {
      return null;
    }

    // Get current accessories
    const currentParams = (cat.spriteParams as Record<string, unknown> | null) ?? {};
    const currentAccessories = Array.isArray(currentParams.accessories)
      ? currentParams.accessories
      : [];

    // Don't add duplicate accessories
    const availableAccessories = [
      ...EXPLORATION_ACCESSORIES,
      ...(Math.random() < 0.1 ? RARE_ACCESSORIES : []), // 10% chance for rare accessory
    ].filter((acc) => !currentAccessories.includes(acc));

    if (availableAccessories.length === 0) {
      return null; // Already has all accessories
    }

    // Pick a random accessory
    const newAccessory = availableAccessories[
      Math.floor(Math.random() * availableAccessories.length)
    ];

    // Add to accessories array (max 3 accessories)
    const updatedAccessories = [...currentAccessories, newAccessory].slice(0, 3);

    // Update cat sprite params
    await ctx.db.patch(args.catId, {
      spriteParams: {
        ...currentParams,
        accessories: updatedAccessories,
        accessory: updatedAccessories[0], // First accessory for compatibility
      },
    });

    return newAccessory;
  },
});

/**
 * Add a scar from combat
 * Called when a cat loses a combat encounter
 */
export const addCombatScar = internalMutation({
  args: {
    catId: v.id("cats"),
  },
  handler: async (ctx, args) => {
    const cat = await ctx.db.get(args.catId);
    if (!cat || cat.deathTime) {
      return null;
    }

    // Get current scars
    const currentParams = (cat.spriteParams as Record<string, unknown> | null) ?? {};
    const currentScars = Array.isArray(currentParams.scars) ? currentParams.scars : [];

    // Don't add duplicate scars
    const availableScars = COMBAT_SCARS.filter((scar) => !currentScars.includes(scar));

    if (availableScars.length === 0) {
      return null; // Already has all scars (unlikely but possible)
    }

    // Pick a random scar
    const newScar = availableScars[Math.floor(Math.random() * availableScars.length)];

    // Add to scars array (max 2 scars)
    const updatedScars = [...currentScars, newScar].slice(0, 2);

    // Update cat sprite params
    await ctx.db.patch(args.catId, {
      spriteParams: {
        ...currentParams,
        scars: updatedScars,
        scar: updatedScars[0], // First scar for compatibility
      },
    });

    return newScar;
  },
});

