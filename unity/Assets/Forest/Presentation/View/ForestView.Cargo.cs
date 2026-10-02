using System.Collections.Generic;
using System.Linq;
using IdleCatForest.Simulation;
using UnityEngine;

namespace IdleCatForest.Presentation
{
    /// <summary>Goods a cat carries: bulk cargo or the exact item of its current job, shown above the cat.</summary>
    public sealed partial class ForestView
    {
        private static readonly Vector3 CargoOffset = Vector3.up * .6f;

        private void ReconcileCarriedCargo(Village v, int level)
        {
            var carriedItems = v.Items.ToLookup(item => item.LocationId);
            var awaitingPickup = new HashSet<string>(v.Jobs.Where(job => !job.Completed && job.Phase == "item_fetch").Select(job => job.Id));
            foreach (var c in v.Cats.Where(c => c.Alive && c.Position.Level == level))
            {
                if (!entities.TryGetValue("cat:" + c.Id, out var go)) continue;
                if (c.Cargo.Count > 0 && c.Cargo[0].Amount > 0)
                    Entity("cargo:" + c.Id, "cargo_" + c.Cargo[0].Resource.ToLowerInvariant(), go.transform.position + CargoOffset, .3f);
                else if (c.JobId != "" && !awaitingPickup.Contains(c.JobId))
                {
                    var item = carriedItems[c.JobId].FirstOrDefault();
                    if (item != null) Entity("cargo:" + c.Id, ItemCargoAsset(item.Kind), go.transform.position + CargoOffset, .3f);
                }
            }
        }

        /// <summary>Keeps carried goods on their cat between reconciles.</summary>
        private void FollowCarriedCargo(Cat c, GameObject cat)
        {
            if (entities.TryGetValue("cargo:" + c.Id, out var cargo)) cargo.transform.position = cat.transform.position + CargoOffset;
        }

        private static string ItemCargoAsset(string kind) => kind switch
        {
            "tool" => "cargo_tools",
            "weapon" => "cargo_weapons",
            "armor" => "cargo_armor",
            "mug" => "cargo_brew",
            "bowl" => "water_bowl",
            "furniture" => "cargo_lumber",
            "clothing" => "cargo_cloth",
            "trinket" => "cargo_gem",
            "toy" => "cargo_bone",
            "brick" => "cargo_blocks",
            _ => "cargo_materials"
        };
    }
}
