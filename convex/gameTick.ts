/**
 * Main Game Tick Loop
 *
 * Processes game state every 10 seconds.
 * TASK: TICK-001
 */

import { internalMutation } from "./_generated/server";
import { internal } from "./_generated/api";
import { v } from "convex/values";
import {
  decayNeeds,
  applyNeedsDamageOverTime,
  isDead,
  restoreHunger,
  restoreThirst,
  restoreRest,
} from "../lib/game/needs";
import {
  getAgeInHours,
  getLifeStage,
  shouldDieOfOldAge,
} from "../lib/game/age";
import { getAutonomousAction } from "../lib/game/catAI";
import { getOptimalCatForTask, getAssignedCat, getAssignmentTime } from "../lib/game/tasks";
import { calculateCombatResult } from "../lib/game/combat";
import { calculateSkillGain } from "../lib/game/skills";

/**
 * Process a single game tick for a colony.
 */
export const tickColony = internalMutation({
  args: { colonyId: v.id("colonies") },
  handler: async (ctx, args) => {
    const now = Date.now();
    const colony = await ctx.db.get(args.colonyId);
    if (!colony || colony.status === "dead") {
      return;
    }

    // Scale needs/health decay to real elapsed time.
    // Our cron runs every 10 seconds, but our "needs tick unit" is 10 minutes.
    // This prevents colonies from dying in minutes if resources are briefly low.
    const elapsedSeconds = Math.max(1, (now - colony.lastTick) / 1000);
    const needsTickUnits = elapsedSeconds / 600;

    // Get all alive cats
    const aliveCats = await ctx.db
      .query("cats")
      .withIndex("by_colony_alive", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("deathTime"), null))
      .collect();

    if (aliveCats.length === 0) {
      // Colony is dead
      await ctx.db.patch(args.colonyId, {
        status: "dead",
      });
      await ctx.runMutation(internal.events.logEventInternal, {
        colonyId: args.colonyId,
        type: "death",
        message: "The colony has perished. All cats are dead.",
        involvedCatIds: [],
      });
      return;
    }

    // 1. Decay all cat needs
    for (const cat of aliveCats) {
      const decayedNeeds = decayNeeds(cat.needs, needsTickUnits);
      const damagedNeeds = applyNeedsDamageOverTime(decayedNeeds, needsTickUnits);

      await ctx.db.patch(cat._id, {
        needs: damagedNeeds,
      });

      // Check for death from starvation/dehydration
      if (isDead(damagedNeeds)) {
        await ctx.runMutation(internal.cats.killCatInternal, {
          catId: cat._id,
        });
        const cause =
          decayedNeeds.hunger === 0 && decayedNeeds.thirst === 0
            ? "starvation and dehydration"
            : decayedNeeds.hunger === 0
            ? "starvation"
            : "dehydration";
        await ctx.runMutation(internal.events.logEventInternal, {
          colonyId: args.colonyId,
          type: "death",
          message: `${cat.name} has died from ${cause}.`,
          involvedCatIds: [cat._id],
        });
      }
    }

    // Get updated alive cats
    const updatedCats = await ctx.db
      .query("cats")
      .withIndex("by_colony_alive", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("deathTime"), null))
      .collect();

    // If everyone died during needs decay, end the colony and log.
    if (updatedCats.length === 0) {
      await ctx.db.patch(args.colonyId, { status: "dead", lastTick: now });
      await ctx.runMutation(internal.events.logEventInternal, {
        colonyId: args.colonyId,
        type: "death",
        message: "The colony has perished. All cats are dead.",
        involvedCatIds: [],
      });
      return;
    }

    // 2. Process autonomous cat behaviors
    // Keep a local copy so multiple cats can consume correctly.
    let colonyResources = { ...colony.resources };
    const hasBuilding = async (type: string) => {
      const buildings = await ctx.db
        .query("buildings")
        .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
        .collect();
      return buildings.some((b) => b.type === type && b.constructionProgress === 100);
    };

    for (const cat of updatedCats) {
      const hasBeds = await hasBuilding("beds");
      const action = getAutonomousAction(
        cat,
        colonyResources,
        (type) => {
          // Synchronous check - we'll check buildings above
          return false; // Simplified for now
        }
      );

      if (action) {
        switch (action.type) {
          case "eat":
            if (colonyResources.food > 0) {
              const restoredNeeds = restoreHunger(cat.needs);
              await ctx.db.patch(cat._id, { needs: restoredNeeds });
              colonyResources = { ...colonyResources, food: Math.max(0, colonyResources.food - 1) };
            }
            break;
          case "drink":
            if (colonyResources.water > 0) {
              const restoredNeeds = restoreThirst(cat.needs);
              await ctx.db.patch(cat._id, { needs: restoredNeeds });
              colonyResources = { ...colonyResources, water: Math.max(0, colonyResources.water - 1) };
            }
            break;
          case "sleep":
            const sleepNeeds = restoreRest(cat.needs, 20, hasBeds);
            await ctx.db.patch(cat._id, {
              needs: sleepNeeds,
              position: action.position,
            });
            break;
          case "return_to_colony":
            await ctx.db.patch(cat._id, {
              position: { map: "colony", x: 1, y: 1 },
            });
            break;
        }
      }
    }

    // Persist any resource consumption from autonomous actions.
    await ctx.db.patch(args.colonyId, { resources: colonyResources });

    // 2b. Auto-create critical survival tasks if resources are low.
    // Without this, colonies can still stall and starve if the user is idle.
    const existingTasks = await ctx.db
      .query("tasks")
      .withIndex("by_colony", (q) => q.eq("colonyId", args.colonyId))
      .collect();

    const hasTask = (type: string) => existingTasks.some((t) => t.type === type);

    const leader = colony.leaderId ? await ctx.db.get(colony.leaderId) : null;
    const leaderSkill = leader && !leader.deathTime ? leader.stats.leadership : 0;
    const assignmentTime = getAssignmentTime(leaderSkill);

    const createTaskInternal = async (type: any, priority: number) => {
      await ctx.db.insert("tasks", {
        colonyId: args.colonyId,
        type,
        priority,
        assignedCatId: null,
        assignmentCountdown: assignmentTime,
        isOptimalAssignment: false,
        progress: 0,
        createdAt: now,
      });
    };

    if (colonyResources.food <= 3 && !hasTask("hunt")) {
      await createTaskInternal("hunt", 100);
    }
    if (colonyResources.water <= 3 && !hasTask("fetch_water")) {
      await createTaskInternal("fetch_water", 95);
    }
    if (colonyResources.herbs <= 1 && !hasTask("gather_herbs")) {
      await createTaskInternal("gather_herbs", 60);
    }

    // Opportunistic exploration: only when stable and no existing explore task.
    if (
      leaderSkill >= 25 &&
      colonyResources.food >= 6 &&
      colonyResources.water >= 6 &&
      !hasTask("explore") &&
      Math.random() < (leaderSkill / 100) * 0.02
    ) {
      await createTaskInternal("explore", 10);
    }

    // 3. Progress active tasks
    const activeTasks = await ctx.db
      .query("tasks")
      .withIndex("by_colony_assigned", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.neq(q.field("assignedCatId"), null))
      .collect();

    for (const task of activeTasks) {
      if (task.assignedCatId) {
        const assignedCat = await ctx.db.get(task.assignedCatId);
        if (assignedCat && !assignedCat.deathTime) {
          // Progress task based on cat's relevant skill
          const skillValue = (() => {
            switch (task.type) {
              case "hunt":
                return assignedCat.stats.hunting;
              case "fetch_water":
                return assignedCat.stats.vision;
              case "gather_herbs":
                return assignedCat.stats.medicine;
              case "build":
                return assignedCat.stats.building;
              case "clean":
                return assignedCat.stats.cleaning;
              case "guard":
                return assignedCat.stats.defense;
              case "patrol":
                return assignedCat.stats.attack;
              case "teach":
              case "kitsit":
                return assignedCat.stats.leadership;
              case "heal":
                return assignedCat.stats.medicine;
              case "explore":
                return assignedCat.stats.vision;
              case "rest":
                return assignedCat.stats.defense;
              default:
                return assignedCat.stats.hunting;
            }
          })();

          const progressAmount = 4 + skillValue / 12; // feels better across skills
          await ctx.runMutation(internal.tasks.progressTask, {
            taskId: task._id,
            amount: progressAmount,
          });

          // Minimal "movement" for exploration so fog-of-war can progress.
          if (task.type === "explore") {
            const step = () => (Math.random() < 0.5 ? -1 : 1);

            const nextPos =
              assignedCat.position.map !== "world"
                ? { map: "world" as const, x: 6, y: 6 } // Colony center
                : {
                    map: "world" as const,
                    x: assignedCat.position.x + step(),
                    y: assignedCat.position.y + step(),
                  };

            await ctx.db.patch(assignedCat._id, { position: nextPos });

            // Add path wear (this will generate chunk if needed)
            await ctx.runMutation(internal.worldMap.addPathWear, {
              colonyId: args.colonyId,
              x: nextPos.x,
              y: nextPos.y,
              amount: 1,
            });
          }

          // Check if task complete
          const updatedTask = await ctx.db.get(task._id);
          if (updatedTask && updatedTask.progress >= 100) {
            // Apply task rewards to colony resources (simple MVP).
            // NOTE: tasks.ts currently doesn't implement rewards, so without this
            // colonies eventually starve.
            const relevantSkill = (() => {
              switch (task.type) {
                case "hunt":
                  return assignedCat.stats.hunting;
                case "fetch_water":
                  return assignedCat.stats.vision;
                case "gather_herbs":
                  return assignedCat.stats.medicine;
                case "build":
                  return assignedCat.stats.building;
                case "clean":
                  return assignedCat.stats.cleaning;
                case "guard":
                  return assignedCat.stats.defense;
                case "patrol":
                  return assignedCat.stats.attack;
                case "teach":
                case "kitsit":
                  return assignedCat.stats.leadership;
                case "heal":
                  return assignedCat.stats.medicine;
                case "explore":
                  return assignedCat.stats.vision;
                case "rest":
                  return assignedCat.stats.defense;
                default:
                  return assignedCat.stats.hunting;
              }
            })();

            const reward = 1 + Math.floor(relevantSkill / 40);
            if (task.type === "hunt") {
              colonyResources = { ...colonyResources, food: colonyResources.food + reward };
            } else if (task.type === "fetch_water") {
              colonyResources = { ...colonyResources, water: colonyResources.water + reward };
            } else if (task.type === "gather_herbs") {
              colonyResources = { ...colonyResources, herbs: colonyResources.herbs + reward };
            } else if (task.type === "build") {
              colonyResources = { ...colonyResources, materials: colonyResources.materials + reward };
            }

            // Task complete - gain skill XP
            const age = getAgeInHours(assignedCat.birthTime, now);
            const lifeStage = getLifeStage(age);
            const newSkill = calculateSkillGain(
              assignedCat.stats.hunting, // Simplified - use relevant skill
              task.type,
              true, // Task succeeded
              lifeStage
            );

            await ctx.runMutation(internal.cats.updateCatStatsInternal, {
              catId: assignedCat._id,
              stats: {
                ...assignedCat.stats,
                hunting: newSkill, // Update relevant skill
              },
            });

            // Check for accessory discovery during exploration
            if (task.type === "explore") {
              const discoveredAccessory = await ctx.runMutation(
                internal.discoveries.discoverAccessory,
                { catId: assignedCat._id }
              );
              if (discoveredAccessory) {
                await ctx.runMutation(internal.events.logEventInternal, {
                  colonyId: args.colonyId,
                  type: "discovery",
                  message: `${assignedCat.name} found a ${discoveredAccessory} while exploring!`,
                  involvedCatIds: [assignedCat._id],
                });
              }
            }

            await ctx.runMutation(internal.events.logEventInternal, {
              colonyId: args.colonyId,
              type: "task_complete",
              message: `${assignedCat.name} completed ${task.type}.`,
              involvedCatIds: [assignedCat._id],
            });

            await ctx.runMutation(internal.tasks.completeTask, {
              taskId: task._id,
            });
          }
        }
      }
    }

    // Persist task rewards (if any).
    await ctx.db.patch(args.colonyId, { resources: colonyResources });

    // 4. Check leader task assignments
    const pendingTasks = await ctx.db
      .query("tasks")
      .withIndex("by_colony_assigned", (q) => q.eq("colonyId", args.colonyId))
      .filter((q) => q.eq(q.field("assignedCatId"), null))
      .collect();

    if (pendingTasks.length > 0) {
      const leader = colony.leaderId ? await ctx.db.get(colony.leaderId) : null;
      const leaderSkill = leader && !leader.deathTime ? leader.stats.leadership : 0;
      for (const task of pendingTasks) {
          // Update countdown
          const newCountdown = Math.max(0, task.assignmentCountdown - 10);
          await ctx.db.patch(task._id, {
            assignmentCountdown: newCountdown,
          });

          // Auto-assign if countdown reached
          if (newCountdown <= 0) {
            const allCats = await ctx.db
              .query("cats")
              .withIndex("by_colony_alive", (q) => q.eq("colonyId", args.colonyId))
              .filter((q) => q.eq(q.field("deathTime"), null))
              .collect();

            const assignment = getAssignedCat(
              allCats,
              task.type,
              leaderSkill
            );

            if (assignment.cat) {
              await ctx.runMutation(internal.tasks.assignCatToTask, {
                taskId: task._id,
                catId: assignment.cat._id,
                isOptimal: assignment.isOptimal,
              });
            }
          }
      }
    }

    // 5. Process births
    await ctx.runMutation(internal.breeding.processBirths, {
      colonyId: args.colonyId,
    });

    // 6. Process building passive effects
    await ctx.runMutation(internal.buildings.processBuildingEffects, {
      colonyId: args.colonyId,
    });

    // 7. Check for random encounters
    await ctx.runMutation(internal.encounters.checkRandomEncounters, {
      colonyId: args.colonyId,
    });

    // 8. Auto-resolve expired encounters
    await ctx.runMutation(internal.encounters.autoResolveExpired, {
      colonyId: args.colonyId,
    });

    // 9. Update cat ages and check for old age death
    for (const cat of updatedCats) {
      const age = getAgeInHours(cat.birthTime, now);
      const isLeader = colony.leaderId === cat._id;
      const isHealer = cat.stats.medicine > 50;
      const shouldDie = shouldDieOfOldAge(age, isLeader || isHealer);

      if (shouldDie) {
        await ctx.runMutation(internal.cats.killCatInternal, {
          catId: cat._id,
        });
        await ctx.runMutation(internal.events.logEventInternal, {
          colonyId: args.colonyId,
          type: "death",
          message: `${cat.name} has died of old age.`,
          involvedCatIds: [cat._id],
        });
      }
    }

    // 10. Update colony status based on resources
    const finalColony = await ctx.db.get(args.colonyId);
    if (finalColony) {
      const totalResources =
        finalColony.resources.food +
        finalColony.resources.water +
        finalColony.resources.herbs;
      const catCount = updatedCats.length;

      let newStatus = finalColony.status;
      if (catCount === 0) {
        newStatus = "dead";
      } else if (totalResources < 10 && catCount < 3) {
        newStatus = "struggling";
      } else if (totalResources > 50 && catCount > 5) {
        newStatus = "thriving";
      } else {
        newStatus = "starting";
      }

      if (newStatus !== finalColony.status) {
        await ctx.db.patch(args.colonyId, {
          status: newStatus,
        });
        if (newStatus === "dead") {
          await ctx.runMutation(internal.events.logEventInternal, {
            colonyId: args.colonyId,
            type: "death",
            message: "The colony has perished. All cats are dead.",
            involvedCatIds: [],
          });
        }
      }

      // Update last tick
      await ctx.db.patch(args.colonyId, {
        lastTick: now,
      });
    }
  },
});

/**
 * Tick all active colonies.
 */
export const tickAllColonies = internalMutation({
  args: {},
  handler: async (ctx) => {
    const allColonies = await ctx.db
      .query("colonies")
      .withIndex("by_status", (q) => q)
      .collect();

    for (const colony of allColonies) {
      if (colony.status !== "dead") {
        await ctx.runMutation(internal.gameTick.tickColony, {
          colonyId: colony._id,
        });
      }
    }
  },
});



